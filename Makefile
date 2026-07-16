# Turn Craft — 快速测试命令
#
# 使用: make <target>

.PHONY: help check test-core test-platform test-e2e test-ui test-front \
        test-v6-ai test-v6-insights test-v7-llm \
        test-v8-invite test-v8-spectator test-v8-events \
        test-v9-leader test-v9-achieve test-v10-engine test-v10-factory \
        test-all clean

help:
	@grep -E '^[a-zA-Z_-]+:' $(MAKEFILE_LIST) | sort | sed 's/:.*//' | \
		while read t; do printf "  make %-20s # "; grep "^#  $$t " $(MAKEFILE_LIST) || echo ""; done

#  编译检查 — 零 error
check:
	SQLX_OFFLINE=true cargo check -p platform_core -p backend --lib 2>&1

#  核心回归 — 7 项 (并发加入/重连/恢复/事件/槽位/删除)
test-core:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_core.db?mode=rwc" cargo test -p backend --test core_regression -- --test-threads=1

#  平台核心编译 + 测试
test-platform:
	SQLX_OFFLINE=true cargo test -p platform_core

#  E2E 全流程 (需要后端运行)
test-e2e:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_e2e.db?mode=rwc" cargo test -p backend --test e2e_game_flow

#  UI 路径回归测试（无外部依赖）
test-ui:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_ui.db?mode=rwc" cargo test -p backend --test ui_path_regression -- --test-threads=1

#  前端组件测试 (Vitest)
test-front:
	cd front_next && npx vitest run

#  V6 AI 风格测试
test-v6-ai:
	SQLX_OFFLINE=true cargo test -p backend --test v6_ai_style

#  V6 Insights 测试
test-v6-insights:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_v6_insights.db?mode=rwc" cargo test -p backend --test v6_insights_basic -- --test-threads=1

#  V7 LLM Insights 测试
test-v7-llm:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_v7_llm.db?mode=rwc" cargo test -p backend --test v7_insights_llm -- --test-threads=1

#  V8 邀请测试
test-v8-invite:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_v8_invite.db?mode=rwc" cargo test -p backend --test v8_invite -- --test-threads=1

#  V8 观战测试
test-v8-spectator:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_v8_spec.db?mode=rwc" cargo test -p backend --test v8_spectator -- --test-threads=1

#  V8 玩家事件测试
test-v8-events:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_v8_events.db?mode=rwc" cargo test -p backend --test v8_player_event -- --test-threads=1

#  V9 排行榜测试
test-v9-leader:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_v9_leader.db?mode=rwc" cargo test -p backend --test v9_leaderboard -- --test-threads=1

#  V9 成就测试
test-v9-achieve:
	SQLX_OFFLINE=true DATABASE_URL="sqlite://test_v9_achieve.db?mode=rwc" cargo test -p backend --test v9_achievements -- --test-threads=1

#  V10 Blackjack 引擎测试
test-v10-engine:
	SQLX_OFFLINE=true cargo test -p backend --test v10_blackjack_engine -- --test-threads=1

#  V10 Blackjack 工厂测试
test-v10-factory:
	SQLX_OFFLINE=true cargo test -p backend --test v10_blackjack_factory

#  全部核心测试
test-all: test-core test-ui test-v6-ai test-v6-insights test-v7-llm \
          test-v8-invite test-v8-spectator test-v8-events \
          test-v9-leader test-v9-achieve test-v10-engine test-v10-factory \
          test-front

#  清理测试数据库
clean:
	rm -f test_core.db test_ui.db test_v6_insights.db test_v7_llm.db \
	      test_v8_invite.db test_v8_spec.db test_v8_events.db \
	      test_v9_leader.db test_v9_achieve.db app.log
