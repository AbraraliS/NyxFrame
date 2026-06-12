import gi
import os
os.environ["PW_DISABLE_DMABUF"] = "1"
gi.require_version('Gst', '1.0')
from gi.repository import Gst, GLib
Gst.init(None)

pipeline_str = "pipewiresrc ! video/x-raw,format=BGRx ! fakesink dump=true"
print(f"Testing pipeline: {pipeline_str}")
pipeline = Gst.parse_launch(pipeline_str)
pipeline.set_state(Gst.State.PLAYING)

loop = GLib.MainLoop()
def on_message(bus, message):
    if message.type == Gst.MessageType.ERROR:
        err, debug = message.parse_error()
        print(f"Error: {err}, {debug}")
        loop.quit()

bus = pipeline.get_bus()
bus.add_signal_watch()
bus.connect("message", on_message)

GLib.timeout_add_seconds(3, loop.quit)
loop.run()
pipeline.set_state(Gst.State.NULL)
