#!/bin/bash

echo ""
echo ""
echo "     _.-^^---....,,--        "
echo " _--                  --_    "
echo " <                        >) "
echo " |                         | "
echo "  \._                   _./  "
echo "     '''--. . , ; .--'''     "
echo "          | |   |            "
echo "       .-=||  | |=-.         "
echo "        -=#$%&%$#=-'         "
echo "          | ;  :|            "
echo "      .,-#%&$@%#&#~,         "
echo "  -------------------------  "
echo "  NUKING YOUR SAITO INSTALL  "
echo "  -------------------------  "
echo ""

find ../data/blocks -name '*.sai' | xargs rm
find ../data/test/blocks -name '*.sai' | xargs rm
