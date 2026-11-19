#pragma once

#include <memory>
#include <span>

namespace mayo {

/// Streams interleaved float32 PCM to the default playback device.
/// write() and queuedSeconds() are thread-safe, so a decoder thread can feed it directly.
class AudioOutput {
public:
    explicit AudioOutput(int sampleRate = 48000, int channels = 2);
    ~AudioOutput();
    AudioOutput(const AudioOutput&) = delete;
    AudioOutput& operator=(const AudioOutput&) = delete;

    [[nodiscard]] bool isOpen() const;
    [[nodiscard]] int sampleRate() const { return m_rate; }
    [[nodiscard]] int channels() const { return m_channels; }

    void write(std::span<const float> interleaved);
    /// Audio buffered but not yet handed to the device.
    [[nodiscard]] double queuedSeconds() const;
    void clear();
    void pause();
    void resume();
    [[nodiscard]] bool isPaused() const;
    /// Linear gain, 0..1 (values above 1 amplify).
    void setVolume(float volume);

    struct Impl;

private:
    std::unique_ptr<Impl> m_impl;
    int m_rate;
    int m_channels;
};

}  // namespace mayo
