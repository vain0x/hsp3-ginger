#include <cstdio>
#include "app_args.h"

AppArgs app_args_parse(int argc, const char* const* args) {
    auto aa = AppArgs{};

    if (argc == 0) {
        aa.help = true;
    }

    for (int y = 0; y < argc; y++) {
        if (args[y][0] == '-' && args[y][1] == '-') {
            auto flag = args[y];
            if (strcmp(flag, "-h") == 0 || strcmp(flag, "--help") == 0) {
                aa.help = true;
                continue;
            }
            else if (strcmp(flag, "--release") == 0) {
                aa.release = true;
                continue;
            }
            else if (strcmp(flag, "--obj-name") == 0 && y + 1 < argc) {
                y++;
                aa.obj_name = args[y];
                continue;
            }
            else if (strcmp(flag, "--ref-name") == 0 && y + 1 < argc) {
                y++;
                aa.ref_name = args[y];
                continue;
            }
            else if (strcmp(flag, "--hsp") == 0 && y + 1 < argc) {
                y++;
                aa.hsp_path = args[y];
                continue;
            }
            else {
                goto L_ERROR;
            }
        }
        else {
            aa.add_positional(args[y]);
            continue;
        }

    L_ERROR:
        aa.add_error(std::string{ u8"引数を解釈できません: " } +args[y]);
    }

    aa.finish();

    return aa;
}

const char* app_args_usage() {
    return
        u8"usage: hsp3_ginger_compiler [オプション] [ファイル名]\n"
        u8"       --release             リリースモード\n"
        u8"       --obj-name [ファイル名]  オブジェクトファイル名\n"
        u8"       --ref-name [ファイル名]  エントリーポイントの名前\n"
        u8"       --hsp [パス]           HSP ディレクトリを指定\n";
}
