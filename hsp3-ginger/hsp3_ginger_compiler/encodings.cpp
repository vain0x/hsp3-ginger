#include <cassert>
#include <memory>
#include <string>
#include <tchar.h>
#include <vector>
#include "encodings.h"

OsString ansi_to_os_str(const char* ansi_str) {
    assert(ansi_str != nullptr);

    auto ansi_str_len = std::strlen(ansi_str) + 1;

    auto len = MultiByteToWideChar(CP_ACP, 0, ansi_str, (int)ansi_str_len, nullptr, 0);
    assert(len >= 1);

    auto os_str = std::vector<TCHAR>(len, (TCHAR)0);
    MultiByteToWideChar(CP_ACP, 0, ansi_str, (int)ansi_str_len, os_str.data(), len);
    assert(os_str[len - 1] == 0);

    return OsString{ os_str.data() };
}

std::string os_to_utf8_str(LPCTSTR os_str) {
    assert(os_str != nullptr);

    auto os_str_len = _tcslen(os_str) + 1;

    auto len = WideCharToMultiByte(CP_UTF8, 0, os_str, (int)os_str_len, nullptr, 0, nullptr, nullptr);
    assert(len >= 1);

    auto utf8_str = std::vector<char>(len, '\0');
    WideCharToMultiByte(CP_UTF8, 0, os_str, (int)os_str_len, utf8_str.data(), len, nullptr, nullptr);
    assert(utf8_str[len - 1] == 0);

    return std::string{ utf8_str.data() };
}

std::string ansi_to_utf8_str(const char* ansi_str) {
    auto os_str = ansi_to_os_str(ansi_str);
    return os_to_utf8_str(os_str.c_str());
}
