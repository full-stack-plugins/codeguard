# 编译数据库初始化局部验收

对应introduce-rust-codeguard-cli的8.29、15.2。仅接通数据库静态观察，原生项目执行未完成。

实际执行：默认init_command_contract完整回归44项通过，随后新增配置变化/删除与任务保留测试，当前编译数据库定向3项默认/WASM各通过、0忽略。最终WASM全工作区all-targets严格Clippy通过。

公开init只读模式不创建工作区，配置状态unknown、execution为not_run；apply退出3保持partial，把稳定数据库SHA256写入project.json，输入文件不改动，不运行command字符串。command-only数据库阻塞观察完整性。数据库宏变化与删除更新画像，保留用户任务文字；不把参数观察视为标准确认、头文件解析或完整项目覆盖。

当前测试源码SHA256：`db6c07b3d24229a9f3910765981d9439f66e3ba1b851032f98c82fb0e7196733`。

本轮使用CLI临时项目，没有运行原生编译器、clang-tidy或真实安装宿主，不授予生产资格。当前46 partial、26 wasm_candidate_only、292 not_integrated为公开计划中的364条路径；旧52 partial记录为纠正前历史检查点。
