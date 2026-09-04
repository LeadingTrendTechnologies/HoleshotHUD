cmake_minimum_required(VERSION 3.16)
project(mxbo LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(mxbo SHARED
    src/plugin.cpp
    src/state.cpp
    src/config.cpp
    src/shm_writer.cpp
    src/steam_friends.cpp
    src/hud/draw_list.cpp
    src/hud/map_hud.cpp
    src/hud/widgets.cpp
    src/hud/font_atlas.cpp
    src/layout.cpp
    mxbo.def
)

target_include_directories(mxbo PRIVATE src)

set_target_properties(mxbo PROPERTIES
    OUTPUT_NAME "Holeshot-HUD"
    PREFIX ""
    SUFFIX ".dlo"
)

if(MSVC)
    target_compile_options(mxbo PRIVATE /W4 /EHsc /utf-8)
    target_compile_definitions(mxbo PRIVATE _CRT_SECURE_NO_WARNINGS NOMINMAX)
endif()

target_link_libraries(mxbo PRIVATE gdi32)

set(MX_BIKES_DIR "" CACHE PATH "MX Bikes install directory (for post-build copy)")
if(NOT MX_BIKES_DIR AND EXISTS "D:/Steam/steamapps/common/MX Bikes")
    set(MX_BIKES_DIR "D:/Steam/steamapps/common/MX Bikes" CACHE PATH "MX Bikes install directory (for post-build copy)" FORCE)
endif()

if(MX_BIKES_DIR)
    add_custom_command(TARGET mxbo POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E make_directory "${MX_BIKES_DIR}/plugins"
        COMMAND ${CMAKE_COMMAND} -E copy $<TARGET_FILE:mxbo> "${MX_BIKES_DIR}/plugins/Holeshot-HUD.dlo"
        COMMAND ${CMAKE_COMMAND} -E remove -f "${MX_BIKES_DIR}/plugins/mxbo.dlo"
        COMMENT "Installing Holeshot-HUD.dlo into MX Bikes plugins"
    )
endif()
