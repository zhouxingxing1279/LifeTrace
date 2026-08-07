# Anti-Patterns（禁止事项）

LifeTrace 是桌面生产力软件，以下风格一律禁止：

- 大面积渐变、发光、粒子背景、Animated Gradient
- 毛玻璃 / 玻璃拟态滥用
- 超大圆角（全部 16~20px）、无意义胶囊设计
- Card 套 Card 套 Card
- 每个功能一个彩色 Icon Card、Bento Grid 泛滥
- 无意义 Hero、Marketing slogan、大段 AI 生成说明
- 满屏 Loading 转圈、长期占据页面的提示文字
- 用缩小字号解决布局问题
- 随机 margin / padding 与硬编码颜色
- 页面各自实现一套 Empty / Error / Loading
- 正文低于 14px

正确的做法：行式列表、清晰的 Table、语义色、统一 token、克制的空状态。
