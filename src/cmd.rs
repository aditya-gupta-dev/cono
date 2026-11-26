//! Compiler flags. Slices instead of fixed-size arrays, so adding a flag
//! doesn't mean updating a length by hand.

/// Passed to both compilers.
pub const WARN_FLAGS: &[&str] = &[
    "-Wall",
    "-Wextra",
    "-Wpedantic",
    "-Werror",
    "-Wconversion",
    "-Wsign-conversion",
    "-Wshadow",
    "-Wformat=2",
    "-Wundef",
    "-Wnull-dereference",
    "-Wdouble-promotion",
    "-Wimplicit-fallthrough",
    "-Wcast-align",
    "-Wnon-virtual-dtor",
    "-ftrivial-auto-var-init=zero",
    "-fstrict-flex-arrays=3",
];

/// clang++ only.
pub const CLANG_FLAGS: &[&str] = &[
    "-Wdangling",
    "-Wreturn-stack-address",
    "-Wconsumed",
    "-Wthread-safety",
];

/// g++ only.
pub const GPP_FLAGS: &[&str] = &["-fanalyzer", "-Wduplicated-cond", "-Wlogical-op"];

/// Debug mode, both compilers.
pub const DEBUG_FLAGS: &[&str] = &[
    "-O1",
    "-g3",
    "-fno-omit-frame-pointer",
    "-D_GLIBCXX_DEBUG",
    "-D_LIBCPP_HARDENING_MODE=_LIBCPP_HARDENING_MODE_DEBUG",
    "-fsanitize=address,undefined,bounds,null,float-divide-by-zero",
    "-fsanitize-address-use-after-scope",
    "-fno-sanitize-recover=all",
];

/// Debug mode, clang++ only: g++ has no such sanitizers.
pub const CLANG_DEBUG_FLAGS: &[&str] = &["-fsanitize=unsigned-integer-overflow,implicit-conversion"];

/// Release mode, both compilers.
pub const RELEASE_FLAGS: &[&str] = &[
    "-O3",
    "-fno-omit-frame-pointer",
    "-D_GLIBCXX_ASSERTIONS",
    "-D_LIBCPP_HARDENING_MODE=_LIBCPP_HARDENING_MODE_EXTENSIVE",
    "-fsanitize=thread,undefined",
    "-fno-sanitize-recover=all",
];
