#!/bin/bash
# Generate test video fixtures for resolvefy tests
# Uses varied pixel formats to test format conversion

FIXTURES_DIR="$(dirname "$0")/fixtures"
mkdir -p "$FIXTURES_DIR"

echo "Generating test fixtures..."

# 1. H264 yuv420p + AAC (baseline - already correct format)
ffmpeg -y -f lavfi -i "testsrc=duration=2:size=320x240:rate=30,format=yuv420p" \
  -f lavfi -i "sine=frequency=440:duration=2:sample_rate=48000" \
  -c:v libx264 -preset ultrafast -crf 28 \
  -c:a aac \
  "$FIXTURES_DIR/h264_aac.mp4" 2>/dev/null

# 2. AV1 yuv420p + Opus (passthrough)
ffmpeg -y -f lavfi -i "testsrc=duration=2:size=320x240:rate=30,format=yuv420p" \
  -f lavfi -i "sine=frequency=440:duration=2:sample_rate=48000" \
  -c:v libsvtav1 -preset 8 -crf 40 \
  -c:a libopus \
  "$FIXTURES_DIR/av1_opus.mp4" 2>/dev/null

# 3. H264 yuv444p + AAC (needs pixel format conversion)
ffmpeg -y -f lavfi -i "testsrc=duration=2:size=320x240:rate=30,format=yuv444p" \
  -f lavfi -i "sine=frequency=440:duration=2:sample_rate=48000" \
  -c:v libx264 -preset ultrafast -crf 28 -pix_fmt yuv444p \
  -c:a aac \
  "$FIXTURES_DIR/h264_aac_yuv444p.mp4" 2>/dev/null

# 4. H264 yuv422p + AAC (needs pixel format conversion)
ffmpeg -y -f lavfi -i "testsrc=duration=2:size=320x240:rate=30,format=yuv422p" \
  -f lavfi -i "sine=frequency=440:duration=2:sample_rate=48000" \
  -c:v libx264 -preset ultrafast -crf 28 -pix_fmt yuv422p \
  -c:a aac \
  "$FIXTURES_DIR/h264_aac_yuv422p.mp4" 2>/dev/null

# 5. H264 yuv444p + Opus (needs pixel format conversion, audio passthrough)
ffmpeg -y -f lavfi -i "testsrc=duration=2:size=320x240:rate=30,format=yuv444p" \
  -f lavfi -i "sine=frequency=440:duration=2:sample_rate=48000" \
  -c:v libx264 -preset ultrafast -crf 28 -pix_fmt yuv444p \
  -c:a libopus \
  "$FIXTURES_DIR/h264_opus_yuv444p.mp4" 2>/dev/null

# 6. Short video for quick tests
ffmpeg -y -f lavfi -i "testsrc=duration=0.5:size=160x120:rate=24,format=yuv420p" \
  -f lavfi -i "sine=frequency=440:duration=0.5:sample_rate=48000" \
  -c:v libx264 -preset ultrafast -crf 28 \
  -c:a aac \
  "$FIXTURES_DIR/short_video.mp4" 2>/dev/null

echo "Done! Generated fixtures in $FIXTURES_DIR"
ls -la "$FIXTURES_DIR"
