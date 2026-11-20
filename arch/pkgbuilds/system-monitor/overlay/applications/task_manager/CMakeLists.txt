# Linux-only: reads /proc and /sys.
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
  # GIO enumerates the installed applications; gdk-pixbuf decodes their icon (PNG or SVG).
  find_package(PkgConfig QUIET)
  if(PkgConfig_FOUND)
    pkg_check_modules(MAYO_ICONS IMPORTED_TARGET gio-2.0 gio-unix-2.0 gdk-pixbuf-2.0)
  endif()
  mayonnaise_app(taskmanager ID io.github.egely1337.Mayonnaise.TaskManager NO_ICON
    SOURCES main.cpp system_stats.cpp process_control.cpp widgets.cpp ../common/app_icons.cpp)
  if(MAYO_ICONS_FOUND)
    target_link_libraries(taskmanager PRIVATE PkgConfig::MAYO_ICONS)
    target_compile_definitions(taskmanager PRIVATE FORMA_HAVE_APP_ICONS=1)
  endif()
  # UI-free executable; the root test list can register it without launching a GUI.
  if(MAYONNAISE_BUILD_TESTS AND TARGET doctest::doctest)
    add_executable(taskmanager_process_tests test_process_control.cpp process_control.cpp system_stats.cpp)
    target_link_libraries(taskmanager_process_tests PRIVATE doctest::doctest)
    target_compile_features(taskmanager_process_tests PRIVATE cxx_std_20)
    mayonnaise_set_warnings(taskmanager_process_tests)
    mayonnaise_set_sanitizers(taskmanager_process_tests)
  endif()
endif()

if(TARGET taskmanager)
  set_target_properties(taskmanager PROPERTIES OUTPUT_NAME system-monitor)
endif()
