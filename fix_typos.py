
import os

file_path = "uploads/riverside_vibe_lab 氛圍實驗室 ep0 - 從 0 開始的 ai 實驗之旅_vibe_lab 氛圍實驗室's st.srt"

replacements = [
    ("VIVE LAB", "Vibe Lab"),
    ("vibe lab 分為實驗室", "Vibe Lab 氛圍實驗室"),
    ("backlight", "Vibe Lab"),
    ("trapGPT", "ChatGPT"),
    ("PoE", "Poe"),
    ("姿勢的巨浪", "知識的巨浪"),
    ("formal 巨獸", "FOMO 巨獸"),
    ("淚眼婆說", "淚眼婆娑"),
    ("何其諸心", "何其誅心"),
    ("撲滿血淚", "鋪滿血淚"),
    ("撲弄", "播弄"),
    ("穩住麻瓜", "文組麻瓜"),
    ("絨物", "冗務"),
    ("AI 降臉", "AI 降臨"),
    ("Libot", "Line Bot"),
    ("男模女類", "男默女淚"),
    ("高靈頓", "高靈通"),
    ("GI 社群", "GAI 社群"),
    ("GI 年會", "GAI 年會"),
    ("MoseKey", "Mosky"),
    ("trapGBT", "ChatGPT"),
    ("track gpt", "ChatGPT"),
    ("ChatGC", "ChatGPT"),
    ("mini journey", "Midjourney"),
    ("李沐淵", "李慕約"),
    ("皮尤", "披露"),
    ("publicity", "Perplexity"),
    ("chopp gpt", "ChatGPT"),
    ("render", "Render"),
    ("processor", "Perplexity"),
    ("superbase", "Supabase"),
    ("part 中", "Python"),
    ("gptx", "GPTs"),
    ("pump", "prompt"),
    ("成績何時", "曾幾何時"),
    ("國人體", "供應鏈體系"),
    ("五輩學院", "五倍學院"),
    ("easy bundle", "Easy Bundle"),
    ("好此", "好死不死"),
    ("錯稱單音", "做成短影音"),
    ("AI formal", "AI FOMO"),
    ("不只貼上", "複製貼上"),
    ("當記者五年", "當街頭藝人五年"),
    ("放機", "放 GitHub"),
    ("開原專案", "開源專案"),
    ("拜克大王", "賣課大王"),
    ("nano banana", "Leonardo.ai"),
    ("share gpt", "ChatGPT"),
    ("320 的素材", "300x250"),
    ("36x280", "336x280"),
    ("ttt 影片", "TikTok 影片"),
    ("line lab 影片", "LINE VOOM 影片"),
    ("gdn", "GDN"),
    ("殭屍", "講師"),
    ("小劇", "小聚"),
]

try:
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    new_content = content
    for old, new in replacements:
        new_content = new_content.replace(old, new)

    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(new_content)

    print(f"Successfully processed {file_path}")

except Exception as e:
    print(f"Error: {e}")
