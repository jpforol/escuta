#!/usr/bin/env bash
# Teste opcional: injeta fala sintética num canal isolado, sem alterar a saída padrão.
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$project_dir"
mkdir -p test-artifacts
sink_name="escuta_teste_$$"
module_id="$(pactl load-module module-null-sink "sink_name=$sink_name")"
listen_pid=''
cleanup() {
    if [[ -n "$listen_pid" ]]; then
        kill "$listen_pid" 2>/dev/null || true
        wait "$listen_pid" 2>/dev/null || true
    fi
    pactl unload-module "$module_id"
}
trap cleanup EXIT
espeak-ng -v pt-br -s 145 -w test-artifacts/fluxo.wav \
    'Esta é uma aula de programação. Vamos estudar variáveis e funções.'
./iniciar.sh listen --source "$sink_name.monitor" --seconds 14 \
    > test-artifacts/fluxo.txt 2> test-artifacts/fluxo.log &
listen_pid=$!
ready=0
for attempt in {1..100}; do
    if rg -q 'Capturando áudio' test-artifacts/fluxo.log; then ready=1; break; fi
    if ! kill -0 "$listen_pid" 2>/dev/null; then break; fi
    sleep 0.1
done
if [[ "$ready" != 1 ]]; then
    cat test-artifacts/fluxo.log >&2
    exit 1
fi
sleep 0.5
paplay --device="$sink_name" test-artifacts/fluxo.wav
wait "$listen_pid"
listen_pid=''
test -s test-artifacts/fluxo.txt
cat test-artifacts/fluxo.txt
rg 'Bloco' test-artifacts/fluxo.log
