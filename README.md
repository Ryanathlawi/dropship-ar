<div dir="rtl">

# OW2 // DROPSHIP — النسخة العربية

> [!IMPORTANT]
> **هذا المشروع ليس من تطويري.**
> البرنامج الأصلي هو [stowmyy/dropship](https://github.com/stowmyy/dropship) من تطوير [**@stowmyy**](https://github.com/stowmyy) (stormy)، وكل الفضل والشكر له.
> هذا المستودع مجرد **نسخة معرّبة** من الإصدار [v3.0.6](https://github.com/stowmyy/dropship/releases/tag/v3.0.6): واجهة عربية بالكامل، لا أكثر.

برنامج محمول (ملف `exe` واحد بدون تثبيت) يخليك تختار سيرفرات **أوفرواتش 2** اللي تلعب عليها، بحظر السيرفرات اللي ما تبيها عن طريق قواعد جدار حماية ويندوز — بدون تعديل أي ملف من ملفات اللعبة.

## التحميل

| الملف | الوصف |
| -- | -- |
| [**dropship-ar.exe**](../../releases/latest/download/dropship-ar.exe) | النسخة العادية — نفس البرنامج الأصلي بالضبط لكن بالعربي |
| [**dropship-ar-animated.exe**](../../releases/latest/download/dropship-ar-animated.exe) | النسخة المتحركة — نفس الشيء مع أنميشنات خفيفة (تظهر عند التفاعل فقط ولا تستهلك المعالج وقت اللعب) |
| [**الكود المصدري (zip)**](../../archive/refs/heads/main.zip) | المشروع كامل للبناء بنفسك، وكل الإصدارات في صفحة [الإصدارات](../../releases) |

الملفان مبنيان من نفس الكود؛ الفرق فقط خيار البناء `--features animations`.

## وش الفرق عن النسخة الأصلية؟

- **الواجهة عربية بالكامل**: كل القوائم والأزرار والرسائل والسجل، مع تخطيط معكوس (من اليمين لليسار).
- **خط ثمانية** ([font.thmanyah.com](https://font.thmanyah.com/)) لكل النصوص، مدمج داخل الـ exe.
- **جولة تعريفية (Product Tour)** تشرح كل جزء من البرنامج خطوة بخطوة عند أول تشغيل (تنقّل بالأزرار أو Enter/الأسهم، و Esc للتخطي)، ويمكن إعادتها من تبويب «الخيارات» أو «المساعدة».
- **ألوان علم المملكة العربية السعودية** (الأخضر والأبيض) في الوضعين الفاتح والداكن.
- **نسخة متحركة اختيارية**: نبض حول عنصر الجولة، تدرّج لون أزرار السيرفرات والنجمة تنط عند الحظر، خط ينزلق تحت التبويب النشط، انزلاق صفحات الترحيب، وتلاشي رسائل الحالة.
- دعم كتابة العربية داخل محرك `egui` (تشكيل الحروف واتجاه النص) عبر تعديل صغير على مكتبة `epaint` (مجلد `dropship/vendor/epaint`).

كل ما عدا ذلك (طريقة الحظر، قائمة السيرفرات، التحديثات) هو نفس عمل المطور الأصلي، والبرنامج يعتمد على نفس [قائمة السيرفرات](https://stowmyy.github.io/dropship/ips.json) التي يحدّثها.

## طريقة الاستخدام

1. حمّل أحد الملفين من [التحميل](#التحميل) وشغّله (يطلب صلاحيات المسؤول لأنه يعدّل جدار الحماية).
2. من قائمة «أبي ألعب على..» اضغط على أي سيرفر لحظره — يبقى محظورًا **حتى بعد إغلاق البرنامج** إلى أن تلغي الحظر.
3. خلاص :]

| الحالة | المعنى |
| -- | -- |
| **مسموح** | كل السيرفرات مسموحة افتراضيًا، وأوفرواتش قد يتصل بها |
| **محظور** | أوفرواتش لن يتصل بهذا السيرفر |
| **بانتظار إغلاق اللعبة** | لا يمكن تغيير الحظر واللعبة مفتوحة؛ أغلق اللعبة والبرنامج مفتوح |

> [!WARNING]
> **تحذيرات الانقطاع**
> 1. **لا تدخل الطابور مع أشخاص آخرين** إلا إذا كانوا يحظرون نفس السيرفرات.
> 2. الحظر يؤثر أيضًا على **الغير مصنّف** و**الألعاب المخصصة**.
> 3. إذا فشل الاتصال بسيرفر، اضغط زر **«تعطيل dropship»** بسرعة لتتجنب حظر التنافسي.

## أسئلة شائعة

| السؤال | الجواب |
| :-- | :-- |
| هل لازم أبقي البرنامج مفتوح؟ | لا، الحظر دائم إلى أن تلغيه (أو اختر «فقط والبرنامج مفتوح» من الخيارات) |
| كيف أحذف البرنامج؟ | اضغط «تعطيل dropship» ثم احذف ملف الـ `exe` |
| هل يثبّت شيء على الجهاز؟ | لا، ملف محمول واحد فقط |
| هل يعدّل اللعبة؟ | **لا**، يضيف قواعد جدار حماية ويندوز فقط، مثل مانع الإعلانات |
| هل فيه فايروس؟ | لا، الكود كله هنا والـ `exe` يُبنى تلقائيًا في [GitHub Actions](../../actions) |
| ما هو `tunneling`؟ | يحصر الحظر على تطبيق اللعبة فقط بدل الجهاز كله، لكي لا تتأثر ألعاب وبرامج أخرى |

## البناء من المصدر

يحتاج [Rust nightly](https://rustup.rs/) وأدوات بناء MSVC (Visual Studio Build Tools مع C++).

```bash
cd dropship
cargo build --release                        # النسخة العادية
cargo build --release --features animations  # النسخة المتحركة
```

### خط ثمانية

خط ثمانية من تصميم وملكية شركة [ثمانية](https://font.thmanyah.com/) ومتاح مجانًا للاستخدام الشخصي والتجاري؛ نسخة الوزن Medium مضمّنة في `dropship/assets/fonts/Thmanyah/` مع ملف ترخيصها لتُدمج داخل الـ `exe` وقت البناء. إذا حُذف ملف الخط يُستخدم [IBM Plex Sans Arabic](https://github.com/IBM/plex) (ترخيص OFL) بدلًا منه تلقائيًا.

## الدعم الفني

- مشكلة في **التعريب** أو الجولة التعريفية؟ افتح [issue](../../issues) هنا.
- مشكلة في **عمل البرنامج نفسه** (انقطاع، سيرفرات جديدة، تحديثات)؟ تواصل مع المطور الأصلي عبر [ديسكورد](https://discord.stormy.gg/) أو [تويتر](https://twitter.stormy.gg/) — هو صاحب المشروع وهو من يصون قائمة السيرفرات.

## الترخيص

[GPL-3.0](LICENSE) — نفس ترخيص المشروع الأصلي.

</div>

---

## A note and a thank-you to the original author (English)

**This project is not mine.** It is an Arabic-only localization of [stowmyy/dropship](https://github.com/stowmyy/dropship) by [@stowmyy](https://github.com/stowmyy) (stormy), based on release v3.0.6.

Everything that makes dropship work — the WFP/firewall logic, the server list, the update mechanism, the whole idea — is stormy's work. All I did was:

- translate the interface into Arabic and mirror the layout for right-to-left reading,
- switch the UI font to *Thmanyah* (with IBM Plex Sans Arabic as an OFL fallback),
- add a step-by-step onboarding tour in Arabic,
- restyle the colors after the Saudi flag,
- offer a second build flavour with light, interaction-only animations,
- and patch the vendored `epaint` crate with UAX#9 bidi so Arabic text shapes and orders correctly in egui.

Thank you, stormy, for building and maintaining dropship in the open, for keeping it portable and simple, and for licensing it under the GPL so that people who play in other languages can enjoy it too. If you'd rather this fork carried a different name or notice, open an issue and I'll change it.

This fork is released under the same GPL-3.0 license as the original.
