#pragma once
#include <vector>

struct AudioFrame {
    std::vector<float> samples; // 20 ms @ 16kHz = 320 samples
};
