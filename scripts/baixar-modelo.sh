#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
case "${1:-turbo}" in
    turbo)
        filename='ggml-large-v3-turbo-q5_0.bin'
        checksum='394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2'
        ;;
    small)
        filename='ggml-small-q5_1.bin'
        checksum='ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb'
        ;;
    *) echo 'Uso: baixar-modelo.sh [turbo|small]' >&2; exit 2 ;;
esac
model_path="$project_dir/models/$filename"
mkdir -p "$project_dir/models"
if [[ -s "$model_path" ]]; then
    echo "Modelo já disponível: $model_path"
    exit 0
fi
curl -fL --retry 2 -o "$model_path.part" \
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/$filename"
printf '%s  %s\n' "$checksum" "$model_path.part" | sha256sum --check --status
mv -- "$model_path.part" "$model_path"
