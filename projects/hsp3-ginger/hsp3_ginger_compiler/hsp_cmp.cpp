#include <cassert>
#include <vector>
#include "hsp_cmp.h"

using HspType0Fn = BOOL(WINAPI*)(int p1, int p2, int p3, int p4);
using HspType1Fn = BOOL(WINAPI*)(char* p1, int p2, int p3, int p4);
using HspType1IntFn = BOOL(WINAPI*)(int* p1, int p2, int p3, int p4);
using HspType5Fn = BOOL(WINAPI*)(char* p1, char* p2, int p3, int p4);
using HspType6Fn = BOOL(WINAPI*)(BMSCR* bm, char* p1, int p2, int p3);
using HspType16Fn = BOOL(WINAPI*)(int p1, int p2, int p3, char* p4);

template <typename Fn>
static auto load_fn(HMODULE handle, const char* fn_name) -> Fn {
    auto fn = GetProcAddress(handle, fn_name);
    assert(fn != nullptr);
    return (Fn)fn;
}

HspCmp::HspCmp(LPCTSTR dll_file_path, const char* src_file_path) {
    auto handle = LoadLibrary(dll_file_path);
    this->handle_ = ModuleHandle{ handle };

    this->hsc_ini = load_fn<HspType6Fn>(handle, "_hsc_ini@16");
    this->hsc_refname = load_fn<HspType6Fn>(handle, "_hsc_refname@16");
    this->hsc_objname = load_fn<HspType6Fn>(handle, "_hsc_objname@16");
    this->hsc_ver = load_fn<HspType16Fn>(handle, "_hsc_ver@16");
    this->hsc_bye = load_fn<HspType0Fn>(handle, "_hsc_bye@16");
    this->hsc_getmes = load_fn<HspType1Fn>(handle, "_hsc_getmes@16");
    this->hsc_clrmes = load_fn<HspType0Fn>(handle, "_hsc_clrmes@16");
    this->hsc_compath = load_fn<HspType6Fn>(handle, "_hsc_compath@16");
    this->hsc_comp = load_fn<HspType0Fn>(handle, "_hsc_comp@16");
    this->hsc3_getsym = load_fn<HspType0Fn>(handle, "_hsc3_getsym@16");
    this->hsc3_messize = load_fn<HspType1IntFn>(handle, "_hsc3_messize@16");
    this->hsc3_make = load_fn<HspType6Fn>(handle, "_hsc3_make@16");
    this->hsc3_getruntime = load_fn<HspType5Fn>(handle, "_hsc3_getruntime@16");
    this->hsc3_run = load_fn<HspType1Fn>(handle, "_hsc3_run@16");

    this->init(src_file_path);
}

HspCmp::~HspCmp() { this->bye(); }

void HspCmp::init(const char* src_file_path) {
    auto p1 = const_cast<char*>(src_file_path);
    this->hsc_ini(nullptr, p1, 0, 0);
}

void HspCmp::bye() { this->hsc_bye(0, 0, 0, 0); }

void HspCmp::set_ref_name(const char* ref_name) {
    auto p1 = const_cast<char*>(ref_name);
    this->hsc_refname(nullptr, p1, 0, 0);
}

void HspCmp::set_obj_name(const char* obj_name) {
    auto p1 = const_cast<char*>(obj_name);
    this->hsc_objname(nullptr, p1, 0, 0);
}

auto HspCmp::get_version() -> std::string {
    char buffer[1024]{};
    this->hsc_ver(0, 0, 0, buffer);
    return std::string{ buffer };
}

auto HspCmp::get_message() -> std::string {
    int message_size = 0;
    this->hsc3_messize(&message_size, 0, 0, 0);
    assert(message_size >= 0);

    auto buffer = std::vector<char>(message_size);

    this->hsc_getmes(buffer.data(), 0, 0, 0);
    buffer[message_size - 1] = 0;

    return std::string{ buffer.data() };
}

void HspCmp::clear_message() { this->hsc_clrmes(0, 0, 0, 0); }

void HspCmp::set_common_path(const char* common_path) {
    auto p1 = const_cast<char*>(common_path);
    this->hsc_compath(nullptr, p1, 0, 0);
}

bool HspCmp::compile(HspCmpOptions options) {
    return this->hsc_comp(options.to_compile_flags(),
        options.to_preprocessor_flags(),
        options.to_debug_window_flags(), 0) == 0;
}

auto HspCmp::get_symbols() -> std::string {
    this->hsc3_getsym(0, 0, 0, 0);
    return this->get_message();
}

auto HspCmp::get_runtime(const char* obj_name) -> std::string {
    const auto BUFFER_SIZE = 1024;
    char name[BUFFER_SIZE]{};

    this->hsc3_getruntime(name, const_cast<char*>(obj_name), 0, 0);
    name[BUFFER_SIZE - 1] = 0;

    auto runtime_name = std::string{ name };
    if (runtime_name == "") {
        runtime_name = std::string{ "hsp3" };
    }
    return runtime_name;
}

bool HspCmp::make(const char* runtime_dir) {
    return this->hsc3_make(nullptr, const_cast<char*>(runtime_dir), 0, 0) == 0;
}

bool HspCmp::run(const char* command) {
    return this->hsc3_run(const_cast<char*>(command), 0, 0, 0) == 0;
}

void HspCmpOptions::set_release_mode(bool release_mode) {
    this->debug_mode = !release_mode;
    this->open_debug_window = !release_mode;
}

int HspCmpOptions::to_compile_flags() {
    int flags = 0;
    if (this->debug_mode) {
        flags |= 1;
    }
    if (this->preprocess_only) {
        flags |= 2;
    }
    if (this->output_utf8) {
        flags |= 4;
    }
    return flags;
}

int HspCmpOptions::to_preprocessor_flags() {
    int flags = 0;
    if (this->no_hspdef) {
        flags |= 1;
    }
    if (this->input_utf8) {
        flags |= 32;
    }
    return flags;
}

int HspCmpOptions::to_debug_window_flags() {
    int flags = 0;
    if (this->open_debug_window) {
        flags |= 1;
    }
    return flags;
}
