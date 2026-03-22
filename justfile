cf-stock:
	@scripts/cf_stock.sh

cf-logs kind="all":
	@scripts/cf_logs.sh --kind "{{kind}}"

cf-logs-follow kind="kernel":
	@scripts/cf_logs.sh --follow --kind "{{kind}}"

cf-kill:
	@scripts/cf_kill.sh
