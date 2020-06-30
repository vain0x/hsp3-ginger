#include "pch.h"
#include "commands.h"
#include "vartype_int64.h"

int WINAPI DllMain(HINSTANCE, DWORD, PVOID) {
	return TRUE;
}

EXPORT void WINAPI hsp3_plugin_init(HSP3TYPEINFO* info) {
	hsp3sdk_init(info);

	registvar(-1, vartype_int64_init);

	commands_init(info);
}
