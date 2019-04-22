#ifndef HSP3_GINGER_COMPILER_ENCODINGS_
#define HSP3_GINGER_COMPILER_ENCODINGS_

#include <string>
#include <Windows.h>

using OsString = std::basic_string<TCHAR>;

extern OsString ansi_to_os_str(const char* ansi_str);

extern std::string os_to_utf8_str(LPCTSTR os_str);

extern std::string ansi_to_utf8_str(const char* ansi_str);

#endif
