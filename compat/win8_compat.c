#include <windows.h>

BOOL __wrap_SystemParametersInfoForDpi(UINT uiAction, UINT uiParam, PVOID pvParam, UINT fWinIni, UINT dpi) {
    (void)dpi;
    // Win8 上没有这个函数，降级为普通调用
    return SystemParametersInfoW(uiAction, uiParam, pvParam, fWinIni);
}
