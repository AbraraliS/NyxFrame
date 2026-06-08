import re

file_path = '/home/drstone/Desktop/NyxFrame/android/app/src/main/java/com/nyxframe/app/ui/screens/SettingsScreen.kt'
with open(file_path, 'r') as f:
    content = f.read()

# Pattern to find the section headers
pattern = r'(\s*)Text\(\s*text = "([^"]+)",\s*color = accentCyan,\s*fontSize = 11\.sp,\s*fontWeight = FontWeight\.ExtraBold,\s*letterSpacing = (1\.5|1)\.sp(.*?)\)'

def replacer(match):
    indent = match.group(1)
    text = match.group(2)
    spacing = match.group(3)
    modifier = match.group(4)
    
    # Check if there is a modifier, if so, put it on the Row, not Text
    mod_text = ""
    if "modifier" in modifier:
        mod_text = "," + modifier
    
    return f'''{indent}Row(verticalAlignment = Alignment.CenterVertically{mod_text}) {{{indent}    Box({indent}        modifier = Modifier.height(14.dp).width(3.dp).background(accentCyan){indent}    ){indent}    Spacer(modifier = Modifier.width(8.dp)){indent}    Text({indent}        text = "{text}",{indent}        color = accentCyan,{indent}        fontSize = 11.sp,{indent}        fontWeight = FontWeight.ExtraBold,{indent}        letterSpacing = {spacing}.sp{indent}    ){indent}}}'''

new_content = re.sub(pattern, replacer, content, flags=re.DOTALL)

with open(file_path, 'w') as f:
    f.write(new_content)
print("Updated headers")
