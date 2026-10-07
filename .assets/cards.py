# -*- coding: utf-8 -*-
"""بطاقات README باللغتين

يصلّح نصوص البطاقات العربية، ويطلع من كل وحدة نسخة إنجليزية (`*-en-*.svg`) بنفس الرسم
معكوس عشان تنقرا من اليسار، ويضمّن في كل بطاقة الحروف اللي تستخدمها بس من خط ثمانية

    python .assets/cards.py

يشتغل أكثر من مرة بنفس النتيجة: النص العربي يُعرف بقديمه أو بجديده، والإنجليزي ينبني من العربي
"""
import base64
import copy
import io
import os
import re
import xml.etree.ElementTree as ET

from fontTools import subset
from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
FONT = os.path.join(HERE, "..", "dropship", "assets", "fonts", "Thmanyah", "thmanyahsans-Medium.otf")
NS = "http://www.w3.org/2000/svg"
T = "{%s}" % NS
ET.register_namespace("", NS)

# القديم ← (العربي الجديد، الإنجليزي). اللي ما هو هنا يبقى زي ما هو في اللغتين (أرقام وأسماء ملفات)
TEXT = {
    # البانر
    "// النسخة العربية بالكامل": ("// النسخة العربية", "// ATHLAWI EDITION"),
    "محدد سيرفرات أوفرواتش 2 — بالعربي": ("محدد سيرفرات أوفرواتش 2 — بالعربي والإنجليزي", "Overwatch 2 server selector — in English and Arabic"),
    "اختر السيرفر اللي تلعب عليه، واحظر الباقي": (None, "Pick the server you play on, and block the rest"),
    "خط ثمانية · جولة تعريفية · ألوان العلم السعودي": (None, "Thmanyah font · guided tour · Saudi flag colors"),
    "نسختان: عادية ومتحركة · بدون تثبيت": (None, "Two builds: plain and animated · nothing to install"),
    # العناوين
    "التحميل": (None, "Download"),
    "لقطات من البرنامج": (None, "Screenshots"),
    "وش الفرق عن الأصلي؟": (None, "What's different from the original?"),
    "الجولة التعريفية": (None, "The guided tour"),
    "طريقة الاستخدام": (None, "How to use it"),
    "أسئلة شائعة": (None, "FAQ"),
    "البناء من المصدر": (None, "Building from source"),
    "الأصل والشكر": (None, "Origin and thanks"),
    # التحميل
    "النسخة العادية — نفس البرنامج الأصلي بالضبط، لكن بالعربي": ("النسخة العادية — نفس البرنامج الأصلي، بالعربي والإنجليزي", "The plain build — the same app as the original, in English and Arabic"),
    "النسخة المتحركة — نفس الشيء مع أنميشنات خفيفة تظهر عند التفاعل فقط": (None, "The animated build — the same, with light animations only while you use it"),
    "الكود المصدري": (None, "Source code"),
    "المشروع كامل بصيغة zip للبناء بنفسك أو التعديل عليه": (None, "The whole project as a zip, to build it yourself or change it"),
    "EXE · 9.4 MB": ("EXE · 9.2 MB", None),
    # الجولة
    "تظهر بعد شاشة الترحيب وتحدّد كل عنصر على الشاشة وتشرحه: الخريطة، قائمة السيرفرات، الاختصارات،": (None, "It shows up after the welcome screen and explains every part of the screen: the map, the server list,"),
    "أفضل مسار، الوضع المصغّر، زر رفع الحظر، الألعاب، التبويبات، شريط الحالة، والمفاتيح.": ("أفضل مسار، الوضع المصغّر، زر رفع الحظر، الألعاب، التبويبات، شريط الحالة، والمفاتيح", "the presets, the best route, mini mode, the unblock button, games, the tabs, the status bar and the keys"),
    "تنقّل بالأزرار أو Enter / الأسهم، و Esc للتخطي. تُعاد من «الخيارات» أو «المساعدة».": ("تنقّل بالأزرار أو Enter والأسهم، وEsc للتخطي، وتقدر تعيدها من «الخيارات» أو «المساعدة»", "Move with the buttons or Enter and the arrow keys, Esc skips it, and you can replay it from options or help"),
    # طريقة الاستخدام
    "حمّل أحد الملفين وشغّله": (None, "Download one of the two files and run it"),
    "يطلب صلاحيات المسؤول لأنه يعدّل قواعد جدار حماية ويندوز": (None, "It asks for admin rights because it blocks servers through the Windows firewall"),
    "اضغط على أي سيرفر لحظره": (None, "Click any server to block it"),
    "من الخريطة أو القائمة — الزر الأيمن يختار سيرفرًا واحدًا ويحظر الباقي، أو اضغط اختصارًا جاهزًا": ("من الخريطة أو القائمة، أو اضغط اختصار جاهز يحظر مجموعة سيرفرات مرة وحدة", "From the map or the list, or press a preset that blocks a group of servers at once"),
    "خلاص، اقفل النافذة": (None, "That's it, close the window"),
    "الحظر يبقى شغّالًا حتى بعد إغلاق البرنامج إلى أن تلغيه": (None, "The block keeps working after you close the app, until you lift it"),
    "⚠ لا تدخل الطابور مع أشخاص آخرين إلا إذا كانوا يحظرون نفس السيرفرات، وإذا فشل الاتصال اضغط «ارفع كل الحظر» بسرعة.": ("⚠ لا تدخل الطابور مع أشخاص آخرين إلا إذا كانوا يحظرون نفس السيرفرات، وإذا فشل الاتصال اضغط «ارفع كل الحظر» بسرعة", "⚠ Don't queue with other people unless they block the same servers, and if you can't connect, press “unblock all” right away"),
    # الأسئلة
    "هل لازم أبقي البرنامج مفتوح؟": (None, "Must the app stay open?"),
    "لا، الحظر دائم إلى أن تلغيه (أو اختر «فقط والبرنامج مفتوح» من الخيارات)": ("لا، الحظر دائم إلى أن تلغيه (أو اختر «أثناء التشغيل» تحت أفضل مسار)", "No, blocks stay until you lift them (or pick “while open” under best route)"),
    "كيف أحذفه؟": (None, "How do I remove it?"),
    "اضغط «ارفع كل الحظر» ثم احذف ملف الـ exe — ما يثبّت شيئًا على الجهاز": (None, "Press “unblock all”, then delete the exe — it doesn't install anything"),
    "هل يعدّل اللعبة؟": (None, "Does it change the game?"),
    "لا، يضيف قواعد جدار حماية ويندوز فقط، مثل مانع الإعلانات": (None, "No, it only adds Windows firewall rules, like an ad blocker"),
    "هل فيه فايروس؟": (None, "Is it a virus?"),
    "لا، الكود كله هنا والـ exe يُبنى تلقائيًا في GitHub Actions": (None, "No, all the code is here and the exe is built automatically by GitHub Actions"),
    "وش الفرق بين النسختين؟": (None, "Why two builds?"),
    "نفس الكود؛ في المتحركة تتدفق مسارات الخريطة والنافذة مركّزة فقط": (None, "Same code; in the animated one the map routes flow while the window is focused"),
    # البناء (علامة LRM بعد C++ حتى ما تنقلب ++ في السطر العربي)
    "يحتاج Rust nightly وأدوات بناء MSVC (Visual Studio Build Tools مع C++)": ("يحتاج Rust nightly وأدوات بناء MSVC (Visual Studio Build Tools مع C++‎)", "Needs Rust nightly and the MSVC build tools (Visual Studio Build Tools with C++)"),
    "يحتاج Rust nightly وأدوات بناء MSVC (Visual Studio Build Tools مع C++).": ("يحتاج Rust nightly وأدوات بناء MSVC (Visual Studio Build Tools مع C++‎)", "Needs Rust nightly and the MSVC build tools (Visual Studio Build Tools with C++)"),
    "الخط مضمّن في المستودع؛ إذا حُذف يُستخدم IBM Plex Sans Arabic (OFL) تلقائيًا.": ("الخط مضمّن في المستودع، وإذا انحذف يُستخدم IBM Plex Sans Arabic (OFL) تلقائيًا", "The font is in the repo; if it's removed, IBM Plex Sans Arabic (OFL) is used instead"),
    # الأصل
    "الأصل": (None, "Original"),
    "من تطوير stormy — طريقة الحظر وقائمة السيرفرات والتحديثات": (None, "by stormy — the blocking method, the server list and the updates"),
    "النسخة العربية من تطوير ريان الأثلاوي: الواجهة واللانشر والاختصارات والجولة والموقع والمجتمع، بنفس ترخيص GPL-3.0.": ("النسخة العربية من تطوير ريان الأثلاوي: الواجهة واللانشر والاختصارات والجولة والموقع والمجتمع، بنفس ترخيص GPL-3.0", "The Athlawi edition by Ryan Athlawi: the interface, launcher, presets, tour, website and community, same GPL-3.0"),
    # الأزرار والذيل
    "الكود المصدري (zip)": (None, "Source code (zip)"),
    "صُنع للمجتمع العربي · تطوير ريان الأثلاوي · شكرًا stormy": (None, "Made for the Arabic community · developed by Ryan Athlawi · thank you, stormy"),
    # رأس المميزات
    "الواجهة والتجربة كلها من تطوير ريان الأثلاوي؛ طريقة الحظر وقائمة السيرفرات من dropship الأصلي (stormy).": ("الواجهة والتجربة كلها من تطوير ريان الأثلاوي، وطريقة الحظر وقائمة السيرفرات من dropship الأصلي (stormy)", "Interface and experience by Ryan Athlawi; blocking and the server list come from stormy's original dropship"),
}

# صفوف المميزات تنبني من جديد كل مرة: (عنوان، وصف) بالعربي ثم بالإنجليزي
FEATURES = [
    ("عربي وإنجليزي", "يفتح بلغة جهازك، وتقدر تغيّرها من الخيارات وقت ما تبي",
     "English and Arabic", "Opens in your PC's language, and you can switch it any time in options"),
    ("واجهة «اللانشر» الجديدة", "خريطة عالم حيّة: مسارات من موقعك لكل سيرفر مسموح، أعلام الدول، وقفل ذهبي على الأفضل",
     "The new launcher", "A live world map: routes to every allowed server, flags, and a gold lock on the best"),
    ("اختصارات الحظر", "زر أو مفتاح يطبّق حظرًا جاهزًا (أوروبا = F1)، أو أنشئ اختصارك: اسم ومفتاح وسيرفرات",
     "Blocking presets", "One button or key applies a saved block (Europe = F1), or make your own"),
    ("بنق حيّ لكل السيرفرات", "يتحدّث كل 15 ثانية، ولو صمت عنوان الـ API يقيس داخل شبكة السيرفر نفسها",
     "Live ping", "Every 15 seconds, measured inside the server's own network if its address stays silent"),
    ("حظر البرامج الثانية", "لو بلوكر ثاني ترك حظر في جدار حماية ويندوز، يقولك عنه ويشيله بضغطة",
     "Other blockers", "Shows blocks other blockers left in Windows Firewall and removes them in one click"),
    ("خط ثمانية", "مدمج داخل الـ exe لكل النصوص، من تصميم شركة ثمانية",
     "Thmanyah font", "Built into the exe for all text, designed by Thmanyah"),
    ("جولة تعريفية", "14 خطوة تشرح كل جزء من البرنامج عند أول تشغيل، ويمكن إعادتها من الخيارات",
     "Guided tour", "14 steps that explain every part of the app on first launch, replayable from options"),
    ("لوحة ألوان متناسقة", "تيل على غرافيت في الوضعين الداكن والفاتح، ولا شيء صارخ",
     "Calm palette", "Teal on graphite in dark and light modes, nothing loud"),
    ("نسخة متحركة اختيارية", "المسارات تتدفق والنقاط تنبض والنافذة مركّزة فقط — صفر استهلاك للمعالج وقت اللعب",
     "Animated build", "Routes flow and dots pulse only while the window is focused, zero CPU while you play"),
    ("دعم العربية في egui", "تعديل صغير على مكتبة epaint لتشكيل الحروف واتجاه النص",
     "Arabic in egui", "A small patch to the epaint library to shape Arabic letters and mixed text"),
]
ROW0, ROW = 78, 30  # أول صف والمسافة بين الصفوف

# رسالة الشكر بالإنجليزي في اللغتين: الأسطر تنلف من جديد على العرض
THANKS = [
    "The Athlawi edition of dropship is developed and maintained by Ryan Athlawi: a new launcher interface with a "
    "live world map, blocking presets, English and Arabic in one app, an onboarding tour, the Thmanyah font, a bidi "
    "patch for egui, the website and the Arabic community around it. It is built on stormy's dropship "
    "(stowmyy/dropship), whose firewall logic, server list and updater it still relies on, under the same GPL-3.0 "
    "license. Support it: paypal.com/paypalme/RayanAthlawi · discord.gg/H8sq6Uc3kA",
    None,  # الفقرة الثانية على حالها
    "If you would prefer a different name or notice on this fork, open an issue and I will change it right away. "
    "Arabic support is also offered upstream as a pull request, in case it is useful there. I hope this is the start "
    "of something good between our two communities. Released under the same GPL-3.0 license as the original, with "
    "gratitude.",
]

LABEL_EN = {
    "banner": "dropship — Athlawi edition: an Overwatch 2 server selector, in English and Arabic",
    "features": "Features", "faq": "FAQ", "usage": "How to use it", "tour": "The guided tour",
    "build": "Building from source", "credits": "The original project stowmyy/dropship by stormy",
}

_font = TTFont(FONT)
_cmap, _hmtx, _upm = _font.getBestCmap(), _font["hmtx"], _font["head"].unitsPerEm


def width(s, size, spacing=0.0, mono=False):
    """عرض النص بالبكسل من عروض حروف الخط (العربي بدون تشكيل الحروف، يكفي للمقارنة)"""
    if mono:
        return len(s) * size * 0.6 + spacing * len(s)
    return sum(_hmtx[_cmap.get(ord(c), ".notdef")][0] for c in s) * size / _upm + spacing * len(s)


def wrap(text, size, limit, mono=False):
    lines, line = [], ""
    for word in text.split(" "):
        test = (line + " " + word).strip()
        if line and width(test, size, mono=mono) > limit:
            lines.append(line)
            line = word
        else:
            line = test
    return lines + [line]


def lookup(s):
    """(العربي، الإنجليزي) لنص قديم أو جديد"""
    for old, (new, en) in TEXT.items():
        if s in (old, new):
            return new or old, en or new or old
    return s, s


def main_text(el):
    """النص اللي يتغيّر: نص العنصر نفسه، أو أول tspan (أسطر البانر المتبدّلة)"""
    if el.text and el.text.strip():
        return el, "text"
    for t in el.iter(T + "tspan"):
        if t.text and t.text.strip():
            return t, "text"
    return None, None


def font_face(chars):
    opts = subset.Options()
    opts.flavor = "woff2"
    opts.layout_features = ["*"]
    opts.hinting = False
    opts.notdef_outline = True
    f = subset.load_font(FONT, opts)
    s = subset.Subsetter(opts)
    s.populate(text="".join(sorted(set(chars) | {" "})))
    s.subset(f)
    buf = io.BytesIO()
    subset.save_font(f, buf, opts)
    return base64.b64encode(buf.getvalue()).decode()


def all_text(root):
    return "".join(t for el in root.iter() if el.tag in (T + "text", T + "tspan") for t in (el.text or "",))


def embed(root):
    style = root.find(T + "style")
    chars = all_text(root)
    style.text = re.sub(r"base64,[A-Za-z0-9+/=]+", "base64," + font_face(chars), style.text, count=1)


def resize(root, h):
    """ارتفاع البطاقة كله: الإطار والخلفية والحد والشريط والخطوط المائلة"""
    old = root.get("height")
    root.set("height", str(h))
    root.set("viewBox", f"0 0 {root.get('width')} {h}")
    for el in root.iter():
        if el.tag == T + "rect" and el.get("height") == old:
            el.set("height", str(h))
        elif el.tag == T + "rect" and el.get("height") == str(int(old) - 1):
            el.set("height", str(h - 1))
        elif el.tag == T + "g" and (el.get("transform") or "").startswith("rotate(-22"):
            el.set("transform", f"rotate(-22 170 {h / 2:.1f})")
            for r in el:
                r.set("height", str(h + 210))


def features(root, lang):
    """يبني صفوف المميزات من FEATURES بنفس شكل أول صف: كل صف مجموعة فيها نقطته ونصّيه وتأخير حركته"""
    rows = [g for g in root.iter(T + "g") if g.get("class") == "rw"]
    parent = next(p for p in root.iter() if rows[0] in list(p))
    delay = [float(re.search(r"animation-delay:([\d.]+)s", g.get("style")).group(1)) for g in rows[:2]]
    tmpl = copy.deepcopy(rows[0])
    at = list(parent).index(rows[0])
    for g in rows:
        parent.remove(g)
    for i, row in enumerate(FEATURES):
        g = copy.deepcopy(tmpl)
        g.set("style", f"animation-delay:{delay[0] + (delay[1] - delay[0]) * i:.2f}s")
        dy = ROW * i
        texts = [el for el in g if el.tag == T + "text"]
        for el in g:
            el.set("y", f"{float(el.get('y')) + dy:g}")
        for el in texts:
            k = 0 if el.get("font-weight") == "700" else 1
            el.text = row[k] if lang == "ar" else row[2 + k]
        parent.insert(at + i, g)
    resize(root, ROW0 + ROW * len(FEATURES))


def thanks(root):
    texts = [el for el in root.iter(T + "text") if el.get("x") == "40"]
    paras, cur = [], []
    for el in texts:  # الفقرات تفصلها العناوين (x=56)
        if cur and float(el.get("y")) - float(cur[-1].get("y")) > 30:
            paras.append(cur)
            cur = []
        cur.append(el)
    paras.append(cur)
    for els, text in zip(paras, THANKS):
        if text is None:
            continue
        lines = wrap(text, 13.5, 843, els[0].get("class") == "m")  # 104 حرف مثل الأسطر الأصلية
        assert len(lines) == len(els), (len(lines), len(els), lines)
        for el, line in zip(els, lines):
            el.text = line
    root.set("aria-label", " ".join(el.text for el in root.iter(T + "text") if el.text))


def mirror(parent, w):
    """يعكس الرسم يمين↔يسار ويقلب النصوص: كل شكل في مجموعة معكوسة (حتى الحركة تنعكس معه)،
    والنص ما ينعكس، بس يتحرك مكانه ويقلب طرف التثبيت"""
    for i, el in enumerate(list(parent)):
        tag = el.tag[len(T):]
        if tag in ("defs", "style", "clipPath", "linearGradient", "radialGradient", "pattern"):
            continue
        if tag == "text":
            el.set("x", f"{w - float(el.get('x')):g}")
            if el.get("class") == "a":  # عربي يصير إنجليزي: الاتجاه ينقلب فالتثبيت يبقى
                el.set("class", "f")
            else:
                a = el.get("text-anchor", "start")
                el.set("text-anchor", {"start": "end", "end": "start"}.get(a, a))
        elif tag == "image":  # الشعار ما ينقلب، بس ينتقل
            el.set("x", f"{w - float(el.get('x')) - float(el.get('width')):g}")
        elif tag == "g" and any(e.tag == T + "text" for e in el.iter()):
            assert not el.get("transform"), "مجموعة فيها نص وتحويل"
            mirror(el, w)
        else:
            g = ET.Element(T + "g", {"transform": f"matrix(-1 0 0 1 {w} 0)"})
            parent.remove(el)
            g.append(el)
            parent.insert(i, g)


def check(name, root):
    """نصوص تطلع برّا البطاقة أو تركب على بعض في نفس السطر"""
    w = float(root.get("width"))
    spans = []
    for el in root.iter(T + "text"):
        node, _ = main_text(el)
        if node is None:
            continue
        s = node.text
        size = float(el.get("font-size"))
        wd = width(s, size, float(el.get("letter-spacing") or 0), el.get("class") == "m")
        x, a = float(el.get("x")), el.get("text-anchor", "start")
        if el.get("class") == "a":  # من اليمين: البداية على اليمين
            a = {"start": "end", "end": "start"}.get(a, a)
        lo = {"start": x, "end": x - wd, "middle": x - wd / 2}[a]
        if not any("ln" in (t.get("class") or "") for t in el.iter(T + "tspan")):  # أسطر البانر تتبادل في نفس المكان
            spans.append((float(el.get("y")), lo, lo + wd, s))
        if name.endswith("-en") and re.search("[؀-ۿ]", s):
            print(f"  ! {name}: بدون ترجمة «{s}»")
        if lo < 12 or lo + wd > w - 12:
            print(f"  ! {name}: برّا البطاقة {lo:.0f}..{lo + wd:.0f} «{s}»")
    for i, (y, a0, a1, s) in enumerate(spans):
        for y2, b0, b1, s2 in spans[i + 1:]:
            if abs(y - y2) < 6 and a0 < b1 + 8 and b0 < a1 + 8:
                print(f"  ! {name}: يركب على «{s2}» «{s}»")


def main():
    names = sorted({f[:-len("-dark.svg")] for f in os.listdir(HERE) if f.endswith("-dark.svg")
                    and not f.startswith("architecture") and "-en-" not in f and not f.endswith("-en-dark.svg")})
    for name in names:
        for theme in ("dark", "light"):
            path = os.path.join(HERE, f"{name}-{theme}.svg")
            ar = ET.parse(path).getroot()
            en = None
            if name == "thanks":
                thanks(ar)
            elif name not in ("sec-thanks",):
                en = copy.deepcopy(ar)
                for root, lang in ((ar, "ar"), (en, "en")):
                    for el in root.iter(T + "text"):
                        node, _ = main_text(el)
                        if node is not None:
                            node.text = lookup(node.text)[0 if lang == "ar" else 1]
                    label = root.get("aria-label") or ""
                    for old, (new, eng) in TEXT.items():
                        if new and old in label:
                            label = label.replace(old, new)
                    root.set("aria-label", label)
                if name == "features":
                    features(ar, "ar")
                    features(en, "en")
                if name.startswith("sec-"):  # الخط بعد العنوان يبدأ بعد آخره، والعنوان الإنجليزي أعرض
                    title = next(el for el in en.iter(T + "text") if el.get("class") == "a")
                    rule = next(el for el in en.iter(T + "rect") if el.get("fill") == "url(#rule)")
                    end = float(title.get("x")) - width(title.text, float(title.get("font-size"))) * 1.05 - 22
                    rule.set("width", f"{end - float(rule.get('x')):.0f}")
                mirror(en, float(en.get("width")))
                en.set("aria-label", LABEL_EN.get(name) or " — ".join(
                    n.text for el in en.iter(T + "text") for n in [main_text(el)[0]] if n is not None and n.text.strip())[:300])
            for root, suffix in ((ar, ""), (en, "-en")):
                if root is None:
                    continue
                embed(root)
                if theme == "dark":
                    check(f"{name}{suffix}", root)
                out = os.path.join(HERE, f"{name}{suffix}-{theme}.svg")
                with open(out, "w", encoding="utf-8", newline="\n") as f:
                    f.write(ET.tostring(root, encoding="unicode"))
        print(name)


if __name__ == "__main__":
    main()
