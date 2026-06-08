import os
import re

file_path = '/home/drstone/Desktop/NyxFrame/android/app/src/main/java/com/nyxframe/app/ui/screens/SettingsScreen.kt'
with open(file_path, 'r') as f:
    content = f.read()

# 1. Update "SETTINGS" to "CONFIGURATION" in the header
content = content.replace('text = "SETTINGS"', 'text = "CONFIGURATION"', 1)

# 2. Update section headers (Text with color = accentCyan, fontSize = 11.sp)
# Pattern to match those headers
pattern = r'(Text\(\s*text = "([^"]+)",\s*color = accentCyan,\s*fontSize = 11\.sp,\s*fontWeight = FontWeight\.ExtraBold,\s*letterSpacing = 1(?:\.5)?\.sp)'

replacement = r'''Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(
                                    modifier = Modifier
                                        .height(14.dp)
                                        .width(3.dp)
                                        .background(accentCyan)
                                        .padding(end = 8.dp)
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Text(
                                    text = "\2",
                                    color = accentCyan,
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.ExtraBold,
                                    letterSpacing = 1.sp
                                )
                            }'''

# Replace all occurrences but keep the other modifiers if they existed
# Wait, some have modifier = Modifier.fillMaxWidth().padding(...)
# It's better to just write a simple script that matches exactly the Text block.
