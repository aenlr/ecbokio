#!/bin/sh

SCRIPT_DIR="$(dirname $(readlink -f "$0"))"

# Ersätt med dina uppgifter
export EASYCASHIER_USERNAME=""
export EASYCASHIER_PASSWORD=""
#export EASYCASHIER_COMPANY=""
export BOKIO_API_TOKEN=""
export BOKIO_COMPANY_ID=""

ecbokio=ecbokio
for d in "$SCRIPT_DIR" "$SCRIPT_DIR/../target/debug" "$SCRIPT_DIR/../target/release"; do
    if [ -x "$d/ecbokio" ]; then
        ecbokio="$d/ecbokio"
        break
    fi
done

$ecbokio "$@"
