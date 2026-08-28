# libjpeg-turbo 3.1 assumes CMAKE_SYSTEM_PROCESSOR is non-empty, but recent
# CMake/MSVC generator combinations can leave it unset. ReTheme WE ships only
# an x64 Windows bundle, so make the target architecture explicit.
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_SYSTEM_PROCESSOR AMD64 CACHE STRING "ReTheme WE target architecture" FORCE)
set(WITH_CRT_DLL ON CACHE BOOL "Use the same dynamic MSVC runtime as Rust" FORCE)
