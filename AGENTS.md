# Prisma Engines 工作约定

本文件只记录当前有效的仓库约定，整理日期为 2026-11-02。当前用户指令优先；行为和命令以本次源码、配置及实际验证为准。历史审计中的 UNFIXED、COMPLETE 和通过数不自动代表当前状态。

只有长期规则或入口变化时才更新本文件。实验经过、逐轮验收数字和原始日志索引写入普通文档，避免在两仓复制完整流水。

## 用户约定与工作边界

- 保留用户已有修改、未跟踪文件和 `record/`。未经授权，不提交、推送、发布，不调整驱动 pin、实例配置、服务、角色或权限。此前拒绝推送 rust-kingbase 的决定仍有效。
- 金仓适配默认只改自己的生产路径与必要的共享登记点，不顺带改原生 PostgreSQL/MySQL 生产行为。Oracle 与 MySQL 两模式分别处理，不能因共享协议而路由到错误的方言、类型或目录实现。
- 不在框架额外模拟数据库表达式语义。BLOB→BIT 使用普通 MODIFY 交数据库接受或拒绝；时间→数字直接原生 CAST，不恢复前缀/饱和/sql_mode模拟、`to_char` 时间编码或容量不足删列的补偿。迁移失败必须保留源列和数据。
- SUM 参数只按真实输入类型消歧：INTEGER/BIGINT/FLOAT4/FLOAT8，Decimal使用 FIXED；结果类型由数据库决定，不强制整数转 NUMERIC，也不自行计算聚合结果。
- PSL拒绝 `UnsignedTinyInt`、`UnsignedSmallInt`、`UnsignedMediumInt`，不恢复域、CHECK或保护触发器模拟；`UnsignedInt`/`sys.uint4`、`UnsignedBigInt`/`sys.uint8` 和默认 Boolean保留。
- 旧 unsigned 域/保护触发器只精确识别为 Unsupported；原样模型应可校验并无迁移步骤。旧 INTEGER/域外键类型不兼容时明确要求人工迁移，不隐瞒外键、不伪造等价类型、不生成无效模型。
- 两模式全文输入统一为金仓原生 tsquery，搜索参数原样保留。MySQL visitor 保留 MATCH/AGAINST SQL重写与 simple/GIN配置，不恢复 Boolean Mode解析器或 `PRISMA_MYSQL_BOOLEAN_QUERY` 标记。
- 两模式 JS adapter 的 `executeScript` 均明确不实现；此决定不删除 Rust 原生迁移脚本、普通事务/savepoint或 MySQL占位符扫描。不要为了脚本测试恢复已撤掉的 JS splitter。
- Kingbase MySQL 独立 SQL Array/EnumArray绑定及结果明确拒绝，包括空/null数组和无结果行时的数组元数据。保留标量 JSON数组与 IN展开；底层驱动 catalog数组和 escape hatch不属于已删除接口，不扩展为 PSL ScalarLists。
- 同功能的 SQL策略、类型范围和数据库能力差异不自动算应删的额外功能。SetDefault、宽字符串/Decimal、全文/TEXT/BLOB键、原始 UUID/XML/网络值等按既有适配范围判断。`pgbouncer` 未要求删除，也无真实代理端到端验收。
- TypeScript adapter 布局以同级 prisma-v7 的 adapter-pg为唯一参照，连接处理留在两模式主文件；不要跨仓重新抽取 connection-string模块。

## 仓库入口与架构

- `psl/`：Schema解析、校验与配置；`prisma-fmt/`：格式化/LSP；`schema-engine/`：迁移、反查、描述器；`quaint/`：SQL AST、visitor、native连接与池。
- `query-compiler/`：Rust规划器、core/schema/request-handlers、Wasm和playground；`libs/driver-adapters/`：Rust端JS adapter接口与转换。不存在根级 `driver-adapters/` 入口。
- `query-engine/connector-test-kit-rs` 仍是测试harness的一部分；不能因 Client已转为QC而删除其测试。Client查询执行在相邻 prisma-v7 的 TypeScript runtime/JS adapter中，Schema Engine仍是原生工具。
- PSL已拒绝 datasource `url`、`directUrl`、`shadowDatabaseUrl`，不是待实施事项。连接信息通过CLI datasource覆盖或TestApi提供。不要恢复旧 `prisma.config.ts` 的 `engine: 'classic' | 'js'` 属性。
- 金仓 Schema Engine入口在 `schema-engine/connectors/sql-schema-connector/src/flavour/kingbase_mysql.rs`、`kingbase_oracle.rs` 及各自目录；目录描述器在 `schema-engine/sql-schema-describer/src/kingbase_mysql.rs`、`kingbase_oracle.rs`。
- 驱动别名与版本以根 `Cargo.toml`/`Cargo.lock` 为准；整理时固定为 rust-kingbase `11a0a2ed3286eb791900ff43ea02114a63937e24`。旧记录的2732da19/da927b91不是当前pin；不得据此回退或推送。
- 相邻 TypeScript 仓通常是 `../prisma-v7`，驱动源码通常是 `../rust-kingbase`；先确认实际路径。必要SQL兼容性结论要区分SQL方言、数据库能力和wire codec，核对官方文档或本机定义，并注明版本/模式边界。

## MySQL 模式实现要点

- 共享 `SqlFamily::Mysql` 不等于原生 MySQL连接。自己的visitor先复用Mysql模板，再做必要金仓适配；参数上限按金仓协议限制为32767。不要将协议 OID当MySQL字段代码。
- `schema-engine` 的测试Tag为 `KingbaseMysql`，不冒充 `Mysql`。`libs/test-setup/src/test_api_args.rs` 当前能力为 `Enums | Json`；Oracle为 `CreateDatabase | Enums | Json`。这是测试harness能力，不能混同PSL生产能力或Quaint注册。
- MySQL原生元数据/类型转换在 `quaint/src/connector/kingbase_mysql/native`；Oracle使用自己的目录。两模式参照PG由各自 `url.rs` 解析连接信息，在 `native/mod.rs` 的 `to_config()` 中用 `Config::new()` 逐项构造驱动配置，保留已有连接选项、编码及默认值，不借结构整理增加查询缓存或改变查询描述行为。缺省端口54321必须在空Config上明确设置，不能在已解析Config上追加第二个默认端口。
- native目录SQL使用已验证的sys接口。`sys_class` 是view，其regclass OID不能直接替换关系依赖classid；通过 `sys_get_object_address` 取得正确classid，不硬编码OID。底层驱动PG/sys目录分派独立核对，不能从SQL长得像PG就断定失败。
- 索引保留逻辑名称、实际物理名称和约束ownership。使用稳定物理名与 `prisma:logical-index:` 注释恢复逻辑名；PK逻辑名保持MySQL合同，删除时使用catalog保存的实际constraint名，不猜 `<table>_pkey`。
- 内联unique约束与独立unique索引不能混同；INCLUDE列不是键列，按 `indnkeyatts` 截断；按ownership选择DROP CONSTRAINT或DROP INDEX。排序unique/fulltext需独立创建时，让calculator、differ、renderer保持一致。
- 索引方向按已验证的 `sys_index_column_has_property` 接口读取；prefix语法被连接器拒绝，不向TEXT/BLOB键错误建议长度前缀。MySQL标识符使用正确转义的反引号，不依赖ANSI_QUOTES。
- 默认值只在完整SQL字面量及已证明中性的cast下归一化。表达式、长度变化、截断/舍入等语义cast保留为dbgenerated；单引号只解码一次，普通KES反斜杠原样保留。以实际INSERT结果、反查和no-op验证，不按字符串外观猜值。
- `BIT(1)`宽度结合full_data_type/typmod和OID判断；Decimal别名按描述器合同归一化；information_schema的enum元数据先转换为共享代码期望的文本。不要伪造数据库未自动创建的外键索引。
- MySQL P2002应恢复schema作用域内的逻辑索引名；事务开始前与自动提交失败后的目录读取有不同边界。保留原错误，不能为了诊断破坏事务或拆分迁移脚本。
- 插入ID的Client适配在JS adapter：编译器给显式自增参数加 `PRISMA_MYSQL_AUTOINCREMENT_ID` 元数据，adapter据此避免旧LAST_INSERT_ID覆盖显式ID。金仓Rust Quaint当前未设置last_insert_id，不能套用原生mysql_async的结果行为。

## Oracle 模式实现要点

- 使用独立Oracle visitor、describer、renderer、calculator及 `KingbaseOracleSchemaExt`。catalog参数绑定规范名称，计算schema与目录过滤保持相同大小写映射；保留用户datasource写法及 `@@schema`/schemas一致性，不无条件转大小写。
- 本机C010全大写引号schema折叠与混合大小写保留的实测不能扩大到全部版本或全部标识符规则；不要仅用ALL_OBJECTS显示名称推断sys内部存储。
- 保留 `kingbase-oracle://` 公共scheme；逐项构造驱动Config不需要重写URL协议。URL Debug不能泄露连接/TLS密码；Oracle默认严格TLS，环境端口54325是本机配置，不是产品默认值。
- Prisma Int/BigInt默认NUMBER(10,0)/NUMBER(19,0)，自增通过schema限定的owned sequence及nextval实现，不改成SERIAL/BIGSERIAL以掩盖外键类型不兼容。保留generation来源、实际序列名和共享依赖信息。
- Oracle DATE包含日期与时间；带时区绑定保留offset，连接设置UTC。NUMBER原始结果是Numeric，不为满足测试截断为整数；ROWID二进制codec与Node文本结果是不同合同。
- 原生类型范围遵循当前校验：FLOAT精度1..53、TIMESTAMP最多6；NUMBER合法scale>precision归一化，负scale不能假装与Oracle完全等价。不要宣称未安装的RAW/LONG RAW扩展已支持。
- Oracle JSON使用已验证的JSONB运算与本模式codec；JSON字符串化和路径不能截断、丢失空键或BigInt/Decimal精度。MERGE源参数元组渲染为列列表SELECT，不使用单一复合字段。
- MERGE RETURNING以及更新匹配条件列有明确拒绝边界；SQL数组不支持。隔离级别仅READ COMMITTED/SERIALIZABLE，不虚报共享能力。隐式M2M表的主键合同需与反查恢复一致。
- 实库类型回归放在 `quaint/src/tests/types/kingbase_oracle.rs`，共享功能尽量扩展既有测试并保留原生对照。不要将原生INTEGER identity测试fixture与生产NUMBER自增表示混同。

## 构建与静态检查

从本仓根选择与改动相关的命令；以下是入口，不要求每次全部执行：

```sh
cargo fmt --all -- --check
cargo clippy --all-features --tests -- -D warnings
cargo check -p quaint --no-default-features --features kingbase-mysql-native
cargo build -p schema-engine-cli
make build-schema-wasm
make build-qc-wasm
```

- `make pedantic` 还包含host/Wasm等检查，实际范围以Makefile为准。不要把一次lib检查称为tests/全仓Clippy通过。
- `make test-unit` 明确排除了quaint、sql-schema-describer、sql-migration-tests等crate，不能据此宣称这些测试已跑；feature-unification通过也不能证明最小feature组合通过。
- 金仓native-only/双模式native-only和非native组合按改动需要分别核对，不借all-features掩盖接口条件编译问题。不通过的上游非金仓组合不能悄悄改为通过或扩大本次修复范围。
- Wasm产物按Makefile/build.sh布局输出；跨仓验证前确认TS侧实际加载匹配的schema/QC Wasm与原生引擎。临时测试覆盖需先备份并按字节恢复；不得默认升级依赖或发布包。
- 小范围修改选择适当验证；文档修改做格式、链接、命令/源码一致性检查，不为文档修复启动数据库或重跑历史全量。

## 测试与快照

常用入口：

```sh
cargo test --locked --offline -p psl --all-features
cargo test --locked --offline -p prisma-fmt --features psl/all
cargo test --locked --offline -p query-compiler --all-features
cargo test --locked --offline -p quaint --all-features --lib <filter>
cargo test --locked --offline -p sql-migration-tests --test migration_tests <filter> -- --nocapture --test-threads=1
cargo test --locked --offline -p sql-introspection-tests --test introspection_tests <filter> -- --nocapture --test-threads=1
cargo test --locked --offline -p sql-schema-describer --all-features <filter> -- --nocapture --test-threads=1
```

- `<filter>` 为占位符，先确认当前测试名/入口，不原样执行。离线失败需报告缺少依赖，不能静默改依赖版本；不要让文档示例触发未经授权的安装或实例创建。
- 正常验证清除继承的 `UPDATE_EXPECT`、`INSTA_UPDATE` 等更新开关。`expect-test` 经人工核对后可用 `UPDATE_EXPECT=1`；QC的insta快照使用 `cargo insta` 审核流程或对应 `INSTA_UPDATE` 开关，不能用UPDATE_EXPECT代替。更新后检查diff，再关闭更新重跑。
- 诊断快照颜色取决于TTY/NO_COLOR/CLICOLOR_FORCE，不按provider切换。保持已校验的ANSI完整快照，需要时以 `env -u NO_COLOR -u UPDATE_EXPECT CLICOLOR_FORCE=1` 运行，避免并行测试修改全局环境。
- 不为通过测试弱化断言、自动接受未知快照、增加无根据allow或修改正确原生对照。保留fixture换行/CRLF及原文，不能遇到diff就将用户文件恢复到HEAD。
- Schema测试使用 `TEST_DATABASE_URL` 和必要的 `TEST_SHADOW_DATABASE_URL`；Quaint分别使用 `TEST_KINGBASE_MYSQL`/`TEST_KINGBASE_ORACLE`，不要混同Node侧URI变量。现有配置见 `quaint/.envrc` 和 `.test_database_urls/`，不要把密码复制进文档。
- 金仓建库使用各自 `TestApiArgs::create_kingbase_mysql_database()`/`create_kingbase_oracle_database()` 和正确connector，不能路由到原生MySQL/PG helper。本机完整MySQL Quaint矩阵使用已配置的prisma库；不要假定另一个test_connect库装有相同JSON函数。
- Quaint的dev-dependency会统一启用all-native，筛选名称不等于只编译金仓。`on_kingbase_mysql`可误匹配JSON类型测试；精确选择用完整名称与 `--exact`，按实际测试注册核对漏选项。
- Cargo会将内部“Test skipped”早退计作通过；核对tag、capability、exclude和测试体guard，分别列出DB实际执行、普通Rust、ignored、skip及重复。过滤表达式/源码文件数不能冒充测试函数数。
- Migration目录按时间戳和字典序回放，快速连续创建时用01_init/02_add等有序名称，不加sleep掩盖顺序错误。
- 临时DB/schema采用唯一自有名称，删除前核对归属；保留现有数据库和用户数据，不改服务、许可、角色或实例参数来让测试通过。

## 未决事项与持久记录

- 仍延期：MySQL映射枚举DDL引号边界；人工共享序列ACL/comment/view/function等额外属性保护；DMMF/版本基线；共享identity额外属性；`relationMode=prisma` deleteMany cardinality问题的原生对照与复查。
- Rust QE/QC金仓tag/provider/adapter/executor桥接、query introspection/Schema CLI建库入口以及相邻Client/CLI公共矩阵和guard的共享覆盖仍需按全量盘点逐项处理。不能因独立测试通过就宣称永久接入完整。
- 正式release/Wasm包发布、全Client/TypedSQL及全部feature组合不因历史聚焦回归自动验收；每次报告写清本次实际范围。
- 旧 `/tmp` 日志、旧报告和JSON可能已归档/移除；失效路径不能支撑本次验收，也不能补造原始日志。
