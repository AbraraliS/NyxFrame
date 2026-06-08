package com.nyxframe.app.data.webrtc

import android.media.MediaCodec
import android.media.MediaFormat
import android.util.Log
import android.view.Surface

class H264Decoder(
    private val onResolutionChanged: (width: Int, height: Int) -> Unit,
    private val onFrameDecoded: (byteSize: Int) -> Unit,
    private val onDecoderRecovered: (() -> Unit)? = null  // Optional: request a keyframe from server
) {
    private var mediaCodec: MediaCodec? = null
    private var isInitialized = false
    private var width = 0
    private var height = 0
    private var activeSurface: Surface? = null

    // Consecutive error guard: prevent rapid crash-reinit loops
    private var consecutiveErrors = 0
    private val maxConsecutiveErrors = 5
    private var lastErrorTimeMs = 0L
    private val errorCooldownMs = 2000L

    fun setSurface(surface: Surface?) {
        activeSurface = surface
        val codec = mediaCodec
        if (codec != null && surface != null) {
            try {
                codec.setOutputSurface(surface)
                Log.i("H264Decoder", "Successfully updated MediaCodec output surface.")
            } catch (e: Exception) {
                Log.e("H264Decoder", "Failed to set output surface on active MediaCodec: ${e.message}")
                // Surface update failed — reset decoder so it reinitializes on next frame
                release()
            }
        }
    }

    fun decode(data: ByteArray) {
        val surface = activeSurface ?: return

        // Throttle rapid-fire error loops
        if (consecutiveErrors >= maxConsecutiveErrors) {
            val now = System.currentTimeMillis()
            if (now - lastErrorTimeMs < errorCooldownMs) {
                return
            } else {
                consecutiveErrors = 0
            }
        }

        if (!isInitialized) {
            initDecoder(1920, 1080, surface)
        }

        val codec = mediaCodec ?: return

        try {
            val nals = findNalUnits(data)
            for (nal in nals) {
                val start = nal.first
                val length = nal.second
                if (length <= 0) continue

                // Find the actual start of the NAL payload (after the start code)
                var startCodeLen = 0
                if (length >= 4 && data[start] == 0.toByte() && data[start+1] == 0.toByte() && data[start+2] == 0.toByte() && data[start+3] == 1.toByte()) {
                    startCodeLen = 4
                } else if (length >= 3 && data[start] == 0.toByte() && data[start+1] == 0.toByte() && data[start+2] == 1.toByte()) {
                    startCodeLen = 3
                }

                var flags = 0
                if (startCodeLen > 0 && length > startCodeLen) {
                    val nalType = data[start + startCodeLen].toInt() and 0x1F
                    
                    if (nalType == 7) {
                        Log.i("H264Decoder", "SPS Detected")
                        flags = MediaCodec.BUFFER_FLAG_CODEC_CONFIG
                    } else if (nalType == 8) {
                        Log.i("H264Decoder", "PPS Detected")
                        flags = MediaCodec.BUFFER_FLAG_CODEC_CONFIG
                    } else if (nalType == 5) {
                        Log.i("H264Decoder", "IDR Detected")
                    }
                }

                val inputBufferIndex = codec.dequeueInputBuffer(10_000)
                if (inputBufferIndex >= 0) {
                    val inputBuffer = codec.getInputBuffer(inputBufferIndex)
                    if (inputBuffer != null) {
                        inputBuffer.clear()
                        val safeLen = minOf(length, inputBuffer.capacity())
                        inputBuffer.put(data, start, safeLen)
                        codec.queueInputBuffer(inputBufferIndex, 0, safeLen, System.nanoTime() / 1000, flags)
                    }
                }
            }

            drainOutput(codec, data.size)
            consecutiveErrors = 0

        } catch (e: MediaCodec.CodecException) {
            handleCodecException(e)
        } catch (e: IllegalStateException) {
            handleIllegalStateException(e)
        } catch (e: Exception) {
            handleUnexpectedException(e)
        }
    }

    private fun drainOutput(codec: MediaCodec, originalDataSize: Int) {
        val bufferInfo = MediaCodec.BufferInfo()
        var outputBufferIndex = codec.dequeueOutputBuffer(bufferInfo, 10_000)

        while (outputBufferIndex >= 0) {
            Log.i("H264Decoder", "Output Buffer Produced")
            codec.releaseOutputBuffer(outputBufferIndex, true)
            com.nyxframe.app.data.webrtc.PipelineTracker.logRendered()
            onFrameDecoded(originalDataSize)
            outputBufferIndex = codec.dequeueOutputBuffer(bufferInfo, 0)
        }

        if (outputBufferIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
            Log.i("H264Decoder", "INFO_OUTPUT_FORMAT_CHANGED")
            val newFormat = codec.outputFormat
            val w = newFormat.getInteger(MediaFormat.KEY_WIDTH)
            val h = newFormat.getInteger(MediaFormat.KEY_HEIGHT)
            Log.i("H264Decoder", "Resolution updated to ${w}x${h} from SPS config.")
            width = w
            height = h
            onResolutionChanged(w, h)
            
            // Re-drain just in case there are frames waiting after format change
            drainOutput(codec, originalDataSize)
        } else if (outputBufferIndex == MediaCodec.INFO_TRY_AGAIN_LATER) {
            // No more frames available for output
        }
    }

    private fun findNalUnits(data: ByteArray): List<Pair<Int, Int>> {
        val nals = mutableListOf<Pair<Int, Int>>()
        var start = -1
        val len = data.size
        var i = 0
        while (i < len - 2) {
            if (data[i] == 0.toByte() && data[i+1] == 0.toByte()) {
                if (i < len - 3 && data[i+2] == 0.toByte() && data[i+3] == 1.toByte()) {
                    // 4-byte start code (00 00 00 01)
                    if (start != -1) nals.add(Pair(start, i - start))
                    start = i
                    i += 4 // skip the start code to avoid overlapping matches
                    continue
                } else if (data[i+2] == 1.toByte()) {
                    // 3-byte start code (00 00 01)
                    if (start != -1) nals.add(Pair(start, i - start))
                    start = i
                    i += 3 // skip the start code
                    continue
                }
            }
            i++
        }
        if (start != -1) {
            nals.add(Pair(start, len - start))
        }
        return nals
    }

    private fun handleCodecException(e: MediaCodec.CodecException) {
        consecutiveErrors++
        lastErrorTimeMs = System.currentTimeMillis()
        Log.e("H264Decoder", "MediaCodec.CodecException (error #$consecutiveErrors): ${e.diagnosticInfo}")
        if (e.isRecoverable) {
            try { mediaCodec?.reset() } catch (_: Exception) {}
            Log.w("H264Decoder", "MediaCodec reset attempted for recoverable error.")
        } else {
            Log.i("H264Decoder", "Decoder Recovery Triggered")
            release()
            onDecoderRecovered?.invoke()
        }
    }

    private fun handleIllegalStateException(e: IllegalStateException) {
        consecutiveErrors++
        lastErrorTimeMs = System.currentTimeMillis()
        Log.e("H264Decoder", "IllegalStateException in decode (error #$consecutiveErrors): ${e.message}")
        Log.i("H264Decoder", "Decoder Recovery Triggered")
        release()
        onDecoderRecovered?.invoke()
    }

    private fun handleUnexpectedException(e: Exception) {
        consecutiveErrors++
        lastErrorTimeMs = System.currentTimeMillis()
        Log.e("H264Decoder", "Unexpected decode exception (error #$consecutiveErrors): ${e.message}")
        Log.i("H264Decoder", "Decoder Recovery Triggered")
        release()
        onDecoderRecovered?.invoke()
    }

    private fun initDecoder(w: Int, h: Int, surface: Surface) {
        try {
            width = w
            height = h
            val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, width, height)
            mediaCodec = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
            Log.i("H264Decoder", "Codec Created")
            mediaCodec?.configure(format, surface, null, 0)
            mediaCodec?.start()
            Log.i("H264Decoder", "Codec Started")
            isInitialized = true
            consecutiveErrors = 0
            Log.i("H264Decoder", "MediaCodec H.264 Decoder initialized at ${width}x${height}")
        } catch (e: Exception) {
            Log.e("H264Decoder", "Failed to initialize MediaCodec: ${e.message}")
            mediaCodec?.release()
            mediaCodec = null
            isInitialized = false
        }
    }

    fun release() {
        try {
            if (isInitialized) mediaCodec?.stop()
            mediaCodec?.release()
        } catch (e: Exception) {
            Log.e("H264Decoder", "Error releasing MediaCodec: ${e.message}")
        } finally {
            mediaCodec = null
            isInitialized = false
            activeSurface = null
            consecutiveErrors = 0
            Log.i("H264Decoder", "MediaCodec released and state fully reset.")
        }
    }
}
