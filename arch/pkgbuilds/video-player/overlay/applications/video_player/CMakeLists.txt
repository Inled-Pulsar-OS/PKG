# Needs FFmpeg's development libraries (libavformat/libavcodec/libswscale/libswresample).
# Ubuntu: libavformat-dev libavcodec-dev libswscale-dev libswresample-dev · Fedora: ffmpeg-free-devel
find_package(PkgConfig QUIET)
if(PkgConfig_FOUND)
  pkg_check_modules(MAYO_FFMPEG IMPORTED_TARGET libavformat libavcodec libswscale libswresample libavutil)
endif()
if(NOT MAYO_FFMPEG_FOUND)
  message(STATUS "Mayonnaise: FFmpeg not found, skipping the video player")
  return()
endif()
mayonnaise_app(video_player ID io.github.egely1337.Mayonnaise.VideoPlayer
  SOURCES main.cpp media_player.cpp library.cpp subtitles.cpp gallery.cpp)
target_link_libraries(video_player PRIVATE PkgConfig::MAYO_FFMPEG)
