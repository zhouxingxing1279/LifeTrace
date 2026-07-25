# Life trace 健身 iOS

Life trace 健身是从 Life trace 个人管理系统中拆分出的原生 iOS 训练应用，使用 SwiftUI 和 SwiftData，最低支持 iOS 17。

## 已实现功能

- 873 个中文训练动作首次启动导入
- 按训练分类、目标肌群搜索动作
- 查看动作图片、器械、难度和中文动作要点
- 只能从动作库选择模板动作
- 配置重量、次数、组数和组间休息
- 逐组训练、计时和进度跟踪
- 保存训练历史与汇总统计
- 所有模板和训练记录保存在设备本地数据库

动作资料来自 `yuhonas/free-exercise-db`，采用公共领域许可。首次导入和动作图片需要网络；导入完成后，动作资料、模板与训练历史可从设备本地读取。

## 在 Mac 上打开

1. 安装 Xcode 16 或更新版本。
2. 安装 XcodeGen：`brew install xcodegen`
3. 在本目录运行：`xcodegen generate`
4. 打开生成的 `HengXuFitness.xcodeproj`
5. 在项目签名设置中选择自己的开发团队。
6. 选择 iPhone 模拟器或真机运行。

## 数据存储

应用使用 SwiftData，底层由系统管理 SQLite 数据库，不使用浏览器存储。当前版本是独立的设备本地应用，不会与网页端自动同步。

## 工程结构

- `HengXuFitnessApp.swift`：应用入口和本地数据库容器
- `ExerciseImporter.swift`：公开动作库导入与中文化
- `ExerciseLibraryView.swift`：动作浏览、筛选和详情
- `TemplatesView.swift`：模板管理、动作选择和训练组配置
- `WorkoutSessionView.swift`：逐组训练流程
- `HistoryView.swift`：训练历史与汇总
