# Настройка Whisper.cpp сервера в Ubuntu LXC (Proxmox)

## Обзор
Конфигурация whisper.cpp сервера в LXC контейнере для интеграции с Rust кодовой базой.

## LXC Конфигурация

### 1. Создание LXC контейнера
```bash
# В Proxmox интерфейсе или CLI
pct create 100 local:vztmpl/ubuntu-22.04-standard_22.04-1_amd64.tar.zx \
  --hostname whisper-server \
  --memory 4096 \
  --cores 4 \
  --net0 name=eth0,bridge=vmbr0,ip=192.168.1.100/24,gw=192.168.1.1 \
  --storage local-lvm:32
```

### 2. CPU Pinning (критично для latency)
```bash
# В Proxmox host
# Закрепить CPU cores за LXC
echo "lxc.cgroup.cpuset.cpus = 2-5" >> /etc/pve/lxc/100.conf
echo "lxc.cgroup.cpu.shares = 1024" >> /etc/pve/lxc/100.conf
```

### 3. HugePages для CUDA (если GPU)
```bash
# В LXC контейнере
echo "vm.hugetlb_shm_group = 1000" >> /etc/sysctl.conf
echo "vm.hugepages_treat_as_movable = 0" >> /etc/sysctl.conf
sysctl -p

# Добавить в /etc/security/limits.conf
* soft memlock unlimited
* hard memlock unlimited
```

### 4. NUMA Affinity (GPU + CPU на одном узле)
```bash
# Проверить NUMA топологию
numactl --hardware

# В Proxmox конфиге контейнера
echo "lxc.cgroup.cpuset.mems = 0" >> /etc/pve/lxc/100.conf  # GPU обычно на node 0
```

## Whisper.cpp сборка и оптимизация

### 1. Клонирование и сборка
```bash
cd /opt
git clone https://github.com/ggerganov/whisper.cpp
cd whisper.cpp

# Сборка с CUDA (если есть GPU)
cmake -B build -DGGML_CUDA=ON -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release -j$(nproc)
```

### 2. Latency-first параметры запуска
```bash
# В скрипте запуска stream_server
export WHISPER_PARAMS="\
--threads $(nproc) \
--gpu-layers 50 \
--beam-size 1 \
--best-of 1 \
--temperature 0.0 \
--max-context 512 \
--language ru"

# Запуск
./build/stream_server $WHISPER_PARAMS
```

### 3. GPU Memory оптимизация
```bash
# Проверить CUDA версию
nvidia-smi

# Мониторить GPU память
watch -n 1 nvidia-smi --query-gpu=memory.used,memory.total --format=csv
```

## Мониторинг и отладка

### 1. System monitoring
```bash
# CPU usage per core
mpstat -P ALL 1

# Memory usage
free -h
vmstat 1

# Network latency
ping -c 10 192.168.1.1
```

### 2. Whisper performance logs
```bash
# Включить verbose logging в stream_server
export WHISPER_VERBOSE=1

# Мониторить latency
tail -f /var/log/whisper_server.log | grep "inference_time"
```

### 3. Benchmark script
```bash
# Создать тест аудио
sox -r 16000 -c 1 -b 16 test.wav synth 3 sine 440

# Тестировать latency
time curl -X POST http://localhost:9001/transcribe \
  -H "Content-Type: audio/pcm" \
  --data-binary @test.wav
```

## Production hardening

### 1. Auto-restart
```bash
# Systemd service для stream_server
cat > /etc/systemd/system/whisper-server.service << EOF
[Unit]
Description=Whisper STT Server
After=network.target

[Service]
Type=simple
User=whisper
WorkingDirectory=/opt/whisper.cpp
ExecStart=/opt/whisper.cpp/build/stream_server --threads 4 --gpu-layers 50
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

systemctl enable whisper-server
systemctl start whisper-server
```

### 2. Health checks
```bash
# Проверка что сервер отвечает
curl -f http://localhost:9001/health || systemctl restart whisper-server
```

### 3. Log rotation
```bash
# В /etc/logrotate.d/whisper-server
/var/log/whisper-server.log {
    daily
    rotate 7
    compress
    missingok
    notifempty
}
```

## Troubleshooting

### Высокий latency (>300ms)
1. Проверить CPU pinning
2. Уменьшить gpu-layers если не хватает памяти
3. Проверить NUMA affinity
4. Мониторить контекст switching

### GPU out of memory
1. Уменьшить gpu-layers
2. Проверить HugePages
3. Перезапустить контейнер

### Network issues
1. Проверить MTU settings
2. Мониторить packet loss
3. Проверить firewall rules

## Performance targets

- **Cold start**: <2s
- **First inference**: <300ms
- **Subsequent**: <120ms
- **Memory usage**: <4GB
- **CPU usage**: <80%

## Backup и recovery

```bash
# Backup модели и configs
tar -czf whisper_backup.tar.gz \
  /opt/whisper.cpp/models/ \
  /etc/systemd/system/whisper-server.service \
  /etc/pve/lxc/100.conf
