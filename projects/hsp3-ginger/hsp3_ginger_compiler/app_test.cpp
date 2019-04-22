#ifndef NDEBUG

#include <cassert>
#include <cstdlib>
#include <fstream>
#include <vector>
#include "app_args.h"
#include "app_tests.h"
#include "encodings.h"
#include "hsp_cmp.h"

std::string read_text_file(const char* name) {
    auto file_path = std::string{ "tests/" };
    file_path += name;

    auto fs = std::ifstream{ file_path };
    return std::string{
        std::istreambuf_iterator<char>{fs},
        std::istreambuf_iterator<char>{},
    };
}

HspCmp load_compiler(const char* src_file_path) {
    // 環境変数から hspcmp.dll のパスを構成する。
    auto buffer = std::vector<TCHAR>(1024, 0);
    auto result =
        GetEnvironmentVariable(TEXT("HSP3_GINGER_HSP_PATH"), buffer.data(),
            static_cast<DWORD>(buffer.size()));
    assert(result > 0 && "テストを実行するには、環境変数 HSP3_GINGER_HSP_PATH を設定してください");
    auto hspcmp_path = OsString{ buffer.data() };
    hspcmp_path += TEXT("/hspcmp.dll");

    return HspCmp{ hspcmp_path.data(), src_file_path };
}

void test_ansi_to_os_str() {
    auto ansi_str = read_text_file("nonascii_error.hsp");
    auto os_str = ansi_to_os_str(ansi_str.data());
    assert(os_str.find(TEXT("重複定義")) != OsString::npos);
}

void test_os_to_utf8_str() {
    auto os_str = OsString{ TEXT("こんにちは世界") };
    auto utf8_str = os_to_utf8_str(os_str.c_str());
    assert(utf8_str == "こんにちは世界");
}

void test_compile_error_from_shift_jis_script() {
    auto compiler = load_compiler("tests/nonascii_error.hsp");

    auto success = compiler.compile(HspCmpOptions{});
    assert(!success);

    auto err = ansi_to_os_str(compiler.get_message().c_str());
    assert(err.find(TEXT("[重複定義] in line 4")) != OsString::npos);
}

void test_compiler_error_from_utf8_script() {
    auto compiler = load_compiler("tests/nonascii_error_utf8.hsp");

    auto options = HspCmpOptions{};
    options.input_utf8 = true;
    auto success = compiler.compile(options);
    assert(!success);

    auto err = ansi_to_os_str(compiler.get_message().c_str());
    assert(err.find(TEXT("[重複定義] in line 4")) != OsString::npos);
}

void test_app_args_parse_success() {
    auto args = std::vector<const char*>{ { "--release", "--obj-name", "obj", "src.hsp", "--hsp", "C:/hsp" } };
    auto app_args = app_args_parse(args.size(), args.data());
    assert(app_args.release);
    assert(app_args.obj_name == "obj");
    assert(app_args.src_path == "src.hsp");
    assert(app_args.hsp_path == "C:/hsp");
    assert(!app_args.has_error());
}

void test_app_args_parse_failure() {
    auto args = std::vector<const char*>{};
    auto app_args = app_args_parse(0, args.data());
    assert(app_args.has_error());
}

void app_test() {
    test_ansi_to_os_str();
    test_os_to_utf8_str();
    test_compile_error_from_shift_jis_script();
    test_app_args_parse_success();
    test_app_args_parse_failure();
}

#else

void app_test() {
}

#endif
