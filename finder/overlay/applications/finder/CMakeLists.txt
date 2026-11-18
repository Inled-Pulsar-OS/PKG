if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
  find_package(PkgConfig QUIET)
  if(PkgConfig_FOUND)
    pkg_check_modules(MAYO_FFMPEG IMPORTED_TARGET libavformat libavcodec libswscale libswresample libavutil)
  endif()

  mayonnaise_app(finder ID io.github.egely1337.Mayonnaise.Finder
    SOURCES main.cpp browser.cpp file_icons.cpp file_model.cpp file_ops.cpp panels.cpp preview_cache.cpp tag_manager.cpp views.cpp media_player.cpp)
  if(MAYO_FFMPEG_FOUND)
    target_link_libraries(finder PRIVATE PkgConfig::MAYO_FFMPEG)
    target_compile_definitions(finder PRIVATE MAYO_HAS_FFMPEG=1)
  endif()
endif()
