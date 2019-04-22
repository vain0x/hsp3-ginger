#ifndef HSP3_GINGER_COMPILER_APP_ARGS_
#define HSP3_GINGER_COMPILER_APP_ARGS_

#include <string>
#include <vector>

struct AppArgs {
    bool help;
    bool release;
    bool preprocess_only;
    bool output_utf8;
    bool no_hspdef;
    bool input_utf8;

    std::string src_path;
    std::string hsp_path;
    std::string obj_name;
    std::string ref_name;

    std::vector<std::string> errors;

    bool has_error() const {
        return !this->errors.empty();
    }

    void add_error(std::string error) {
        this->errors.push_back(error);
    }

    void add_positional(std::string arg) {
        if (src_path.empty()) {
            this->src_path = arg;
            return;
        }
        this->add_error(u8"引数が多すぎます: " + arg);
    }

    void finish() {
        if (this->src_path.empty()) {
            this->errors.push_back(u8"スクリプトファイルを指定してください。");
        }
        if (this->hsp_path.empty()) {
            this->errors.push_back(u8"--hsp を指定してください。");
        }
    }
};

extern AppArgs app_args_parse(int argc, const char* const* args);

extern const char* app_args_usage();

#endif
