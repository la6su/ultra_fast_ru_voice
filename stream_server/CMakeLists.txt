cmake_minimum_required(VERSION 3.16)
project(whisper_stream_server LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Корень whisper.cpp
set(WHISPER_ROOT ${CMAKE_CURRENT_SOURCE_DIR}/..)

# ============================================
# Add whisper.cpp as subproject
# ============================================
add_subdirectory(${WHISPER_ROOT} whisper_build)

# ============================================
# Executable (production-ready batch inference server)
# ============================================
add_executable(whisper_stream_server
    whisper_stream_server.cpp
    whisper_streamer.cpp
    # vad.cpp removed - VAD logic is in Rust only
)

# ============================================
# Includes
# ============================================
target_include_directories(whisper_stream_server PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${WHISPER_ROOT}/include
    ${WHISPER_ROOT}/ggml/include
    /opt/uWebSockets/src
    /opt/uSockets/src
)

# ============================================
# Link libraries
# ============================================
target_link_libraries(whisper_stream_server
    PRIVATE
        whisper
        /opt/uSockets/uSockets.a
        pthread
        dl
	z
)
