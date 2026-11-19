#include <SDL3/SDL.h>

#include <atomic>

#include "mayonnaise/audio.hpp"

namespace mayo {

struct AudioOutput::Impl {
    SDL_AudioStream* stream = nullptr;
    std::atomic<bool> paused{false};
};

AudioOutput::AudioOutput(int sampleRate, int channels)
    : m_impl(std::make_unique<Impl>()), m_rate(sampleRate), m_channels(channels) {
    if (!SDL_WasInit(SDL_INIT_AUDIO) && !SDL_InitSubSystem(SDL_INIT_AUDIO)) {
        SDL_Log("mayonnaise: audio init failed: %s", SDL_GetError());
        return;
    }
    SDL_AudioSpec spec{SDL_AUDIO_F32, channels, sampleRate};
    m_impl->stream = SDL_OpenAudioDeviceStream(SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &spec, nullptr, nullptr);
    if (!m_impl->stream) {
        SDL_Log("mayonnaise: could not open audio device: %s", SDL_GetError());
        return;
    }
    SDL_ResumeAudioStreamDevice(m_impl->stream);
}

AudioOutput::~AudioOutput() {
    if (m_impl->stream) SDL_DestroyAudioStream(m_impl->stream);
}

bool AudioOutput::isOpen() const { return m_impl->stream != nullptr; }

void AudioOutput::write(std::span<const float> interleaved) {
    if (!m_impl->stream || interleaved.empty()) return;
    SDL_PutAudioStreamData(m_impl->stream, interleaved.data(), static_cast<int>(interleaved.size_bytes()));
}

double AudioOutput::queuedSeconds() const {
    if (!m_impl->stream) return 0;
    int bytes = SDL_GetAudioStreamQueued(m_impl->stream);
    return bytes <= 0 ? 0.0 : static_cast<double>(bytes) / (sizeof(float) * m_channels * m_rate);
}

void AudioOutput::clear() {
    if (m_impl->stream) SDL_ClearAudioStream(m_impl->stream);
}

void AudioOutput::pause() {
    if (m_impl->stream) SDL_PauseAudioStreamDevice(m_impl->stream);
    m_impl->paused = true;
}

void AudioOutput::resume() {
    if (m_impl->stream) SDL_ResumeAudioStreamDevice(m_impl->stream);
    m_impl->paused = false;
}

bool AudioOutput::isPaused() const { return m_impl->paused; }

void AudioOutput::setVolume(float volume) {
    if (m_impl->stream) SDL_SetAudioStreamGain(m_impl->stream, volume < 0 ? 0 : volume);
}

}  // namespace mayo
