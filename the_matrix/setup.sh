#!/bin/sh

#export LIBGL_ALWAYS_SOFTWARE=1
#export __GLX_VENDOR_LIBRARY_NAME=mesa
#export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.i686.json:/usr/share/vulkan/icd.d/lvp_icd.x86_64.json

Xephyr -br -ac -noreset -screen 1920x1080 :1 & disown
export DISPLAY=:1
qtile start & disown

