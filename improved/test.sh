#!/bin/bash
# Test suite for NextRouter

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_RESULTS=()

# Test framework
run_test() {
    local test_name="$1"
    local test_function="$2"
    
    echo -n "Testing $test_name... "
    
    if $test_function; then
        echo "PASS"
        TEST_RESULTS+=("PASS: $test_name")
        return 0
    else
        echo "FAIL"
        TEST_RESULTS+=("FAIL: $test_name")
        return 1
    fi
}

# Network calculation tests
test_network_calculation() {
    source "$SCRIPT_DIR/utils.sh"
    
    local result
    result=$(calculate_network_details "192.168.1.1/24")
    
    [[ "$result" =~ "NETWORK:192.168.1.0/24" ]] && \
    [[ "$result" =~ "NETMASK:255.255.255.0" ]] && \
    [[ "$result" =~ "BROADCAST:192.168.1.255" ]]
}

# Configuration validation tests
test_config_validation() {
    source "$SCRIPT_DIR/config.sh"
    
    # Test valid configuration
    INTERFACES[WAN0]="eth0"
    INTERFACES[WAN1]="eth1"
    INTERFACES[LAN0]="eth2"
    INTERFACES[LAN_IP]="192.168.1.1/24"
    
    validate_config
}

# Template processing tests
test_template_processing() {
    source "$SCRIPT_DIR/utils.sh"
    
    # Create test template
    cat > /tmp/test_template <<EOF
Network: NETWORK_ADDR
Netmask: NETMASK_VALUE
EOF
    
    declare -A test_vars=(
        ["NETWORK_ADDR"]="192.168.1.0"
        ["NETMASK_VALUE"]="255.255.255.0"
    )
    
    process_template "/tmp/test_template" "/tmp/test_output" test_vars
    
    local content
    content=$(cat /tmp/test_output)
    
    [[ "$content" == *"Network: 192.168.1.0"* ]] && \
    [[ "$content" == *"Netmask: 255.255.255.0"* ]]
}

# Argument parsing tests
test_argument_parsing() {
    source "$SCRIPT_DIR/config.sh"
    source "$SCRIPT_DIR/args.sh"
    
    # Mock arguments
    set -- --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24 --dry-run
    
    parse_arguments "$@"
    
    [[ "${INTERFACES[WAN0]}" == "eth0" ]] && \
    [[ "${INTERFACES[WAN1]}" == "eth1" ]] && \
    [[ "${INTERFACES[LAN0]}" == "eth2" ]] && \
    [[ "${INTERFACES[LAN_IP]}" == "192.168.1.1/24" ]] && \
    [[ "${CONFIG[DRY_RUN]}" == "true" ]]
}

# Dry run tests
test_dry_run_mode() {
    source "$SCRIPT_DIR/config.sh"
    source "$SCRIPT_DIR/utils.sh"
    
    CONFIG[DRY_RUN]="true"
    
    # Should not actually execute
    local output
    output=$(execute "echo 'test command'" 2>&1)
    
    [[ "$output" == *"[DRY RUN]"* ]] && \
    [[ "$output" == *"test command"* ]]
}

# Service management tests (mock)
test_service_management() {
    source "$SCRIPT_DIR/utils.sh"
    
    # Mock systemctl command for testing
    systemctl() {
        case "$1" in
            "is-active")
                return 0  # Always return active for test
                ;;
            *)
                echo "Mock systemctl $*"
                return 0
                ;;
        esac
    }
    
    manage_service "status" "test-service"
}

# Backup and restore tests
test_backup_restore() {
    source "$SCRIPT_DIR/utils.sh"
    
    # Create test files
    sudo mkdir -p /etc/dhcp /etc/prometheus
    sudo touch /etc/dhcp/dhcpd.conf
    sudo touch /etc/prometheus/prometheus.yml
    
    # Test backup
    local backup_dir
    backup_dir=$(create_backup)
    
    [[ -d "$backup_dir" ]] && \
    [[ -f "$backup_dir/dhcpd.conf" ]]
}

# Main test runner
main() {
    echo "Running NextRouter Test Suite"
    echo "============================="
    
    run_test "Network Calculation" test_network_calculation
    run_test "Configuration Validation" test_config_validation  
    run_test "Template Processing" test_template_processing
    run_test "Argument Parsing" test_argument_parsing
    run_test "Dry Run Mode" test_dry_run_mode
    run_test "Service Management" test_service_management
    run_test "Backup/Restore" test_backup_restore
    
    echo
    echo "Test Results:"
    echo "============="
    
    local passed=0
    local failed=0
    
    for result in "${TEST_RESULTS[@]}"; do
        echo "$result"
        if [[ "$result" == PASS:* ]]; then
            ((passed++))
        else
            ((failed++))
        fi
    done
    
    echo
    echo "Summary: $passed passed, $failed failed"
    
    if [[ $failed -eq 0 ]]; then
        echo "All tests passed!"
        exit 0
    else
        echo "Some tests failed!"
        exit 1
    fi
}

main "$@"
