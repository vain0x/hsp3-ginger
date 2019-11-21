#include <cstdio>
#include <Windows.h>
#include "app_args.h"
#include "app_tests.h"
#include "encodings.h"
#include "hsp_cmp.h"

HspCmpOptions app_args_to_options(const AppArgs& app_args) {
    auto options = HspCmpOptions{};
    options.input_utf8 = true;
    options.set_release_mode(app_args.release);
    return options;
}

std::string normalize_eol(const char* str) {
    auto text = std::string{};
    auto i = 0;
    while (str[i] != 0) {
        if (str[i] == '\r' && str[i + 1] == '\n') {
            text.append("\r\n");
            i += 2;
        }
        else if (str[i] == '\n') {
            text.append("\r\n");
            i++;
        }
        else {
            text.push_back(str[i]);
            i++;
        }
    }
    return text;
}

int main(int argc, const char** args) {
    argc--;
    args++;

    if (argc == 0) {
        app_test();
    }

    SetConsoleOutputCP(CP_UTF8);
    setvbuf(stdout, nullptr, _IOFBF, 1024);
    setvbuf(stderr, nullptr, _IOFBF, 1024);

    auto app_args = app_args_parse(argc, args);
    if (app_args.help) {
        fprintf(stderr, "%s", app_args_usage());
        return EXIT_FAILURE;
    }
    if (app_args.has_error()) {
        for (auto error : app_args.errors) {
            fprintf(stderr, "%s\n", error.c_str());
        }
        return EXIT_FAILURE;
    }

    auto options = app_args_to_options(app_args);
    auto hsp_path = ansi_to_os_str(app_args.hsp_path.c_str());
    auto hspcmp_path = hsp_path + TEXT("/hspcmp.dll");
    auto common_path = app_args.hsp_path + "/common/";
    auto src_path = app_args.src_path;
    auto obj_name = app_args.obj_name;
    auto ref_name = app_args.ref_name;

    auto compiler = HspCmp{ hspcmp_path.c_str(), src_path.c_str() };
    {
        compiler.set_common_path(common_path.c_str());

        if (!obj_name.empty()) {
            compiler.set_obj_name(obj_name.c_str());
        }
        if (!ref_name.empty()) {
            compiler.set_ref_name(ref_name.c_str());
        }
    }

    auto success = compiler.compile(options);
    if (!success) {
        auto message = normalize_eol(ansi_to_utf8_str(compiler.get_message().c_str()).c_str());
        fprintf(stderr, "%s", message.c_str());
        return EXIT_FAILURE;
    }

    return EXIT_SUCCESS;
}
