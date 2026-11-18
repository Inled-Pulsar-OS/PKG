if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
  mayonnaise_app(finder ID io.github.egely1337.Mayonnaise.Finder
    SOURCES main.cpp browser.cpp file_icons.cpp file_model.cpp file_ops.cpp panels.cpp preview_cache.cpp tag_manager.cpp views.cpp)
endif()
