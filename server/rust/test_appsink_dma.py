import gi
gi.require_version('Gst', '1.0')
gi.require_version('GstApp', '1.0')
from gi.repository import Gst, GstApp, GLib
Gst.init(None)

pipeline_str = "pipewiresrc ! capsfilter caps=video/x-raw(memory:DMABuf),format=BGRx ! appsink name=sink emit-signals=true max-buffers=1 drop=true"
print(f"Testing pipeline: {pipeline_str}")
pipeline = Gst.parse_launch(pipeline_str)
sink = pipeline.get_by_name("sink")

def on_new_sample(sink):
    sample = sink.emit("pull-sample")
    buf = sample.get_buffer()
    print(f"Got buffer of size {buf.get_size()}")
    success, map_info = buf.map(Gst.MapFlags.READ)
    if success:
        print(f"Mapped successfully, first byte: {map_info.data[0]}")
        buf.unmap(map_info)
    else:
        print("Failed to map buffer")
    return Gst.FlowReturn.OK

sink.connect("new-sample", on_new_sample)
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
