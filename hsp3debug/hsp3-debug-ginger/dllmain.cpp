#include "stdafx.h"
#include "hspsdk/hsp3debug.h"
#include "hspsdk/hsp3struct.h"

#define EXPORT extern "C" __declspec (dllexport)
#define BOOL int

BOOL APIENTRY DllMain(
    HMODULE h_module,
    DWORD  ul_reason_for_call,
    LPVOID reserved
)
{
    switch (ul_reason_for_call)
    {
        case DLL_PROCESS_ATTACH:
            OutputDebugString(TEXT("attach!"));
            break;
        case DLL_THREAD_ATTACH:
        case DLL_THREAD_DETACH:
            break;
        case DLL_PROCESS_DETACH:
            OutputDebugString(TEXT("detach!"));
            break;
    }
    return TRUE;
}

// デバッガーがアタッチされたときに HSP ランタイムから呼ばれる。
EXPORT BOOL APIENTRY debugini(HSP3DEBUG* p1, int p2, int p3, int p4)
{
    OutputDebugString(TEXT("activated"));
    strcat_s(p1->hspctx->refstr, HSPCTX_REFSTR_MAX, "activated");
    return 0;
}

// logmes や assert 0 が実行されたときに HSP ランタイムから呼ばれる。
EXPORT BOOL WINAPI debug_notice(HSP3DEBUG* p1, int p2, int p3, int p4)
{
    OutputDebugString(TEXT("noticed"));
    strcat_s(p1->hspctx->refstr, HSPCTX_REFSTR_MAX, "noticed");

    p1->dbg_set(HSPDEBUG_RUN);
    return 0;
}
