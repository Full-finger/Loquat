#!/bin/sh
# 探测 git bash 的绝对路径，排除 WSL 的 system32 bash 存根；找不到输出空行。
# 供 justfile 的 bash 变量使用；在 Linux 上 powershell.exe 不存在，返回空即走兜底。
powershell.exe -NoProfile -Command "(Get-Command bash -All -ErrorAction SilentlyContinue | Where-Object Source -NotMatch 'system32' | Select-Object -First 1).Source.Trim().Replace('\','/')" 2>/dev/null || true
