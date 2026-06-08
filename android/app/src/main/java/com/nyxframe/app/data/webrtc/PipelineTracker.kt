package com.nyxframe.app.data.webrtc

import android.util.Log

object PipelineTracker {
    var firstFrameReceived = true
    var firstFrameRendered = true

    fun reset() {
        firstFrameReceived = true
        firstFrameRendered = true
    }

    fun logReceived(size: Int) {
        if (firstFrameReceived) {
            Log.i("NyxFramePipeline", "First Binary Packet Received")
            Log.i("NyxFramePipeline", "Packet Size: $size bytes")
            firstFrameReceived = false
        }
    }

    fun logRendered() {
        if (firstFrameRendered) {
            Log.i("NyxFramePipeline", "First Frame Rendered")
            firstFrameRendered = false
        }
    }
}
