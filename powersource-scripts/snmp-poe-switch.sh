#!/usr/bin/env bash
#set -x
readonly IPv4_ADDRESS="10.0.0.51"
readonly USERNAME="cluster-manager"

function on() {
  local _port="${1}"
  snmpset -v1 -c "${USERNAME}" "${IPv4_ADDRESS}" "iso.3.6.1.2.1.105.1.1.1.3.1.${_port}" i 1
}

function off() {
  local _port="${1}"
  snmpset -v1 -c "${USERNAME}" "${IPv4_ADDRESS}" "iso.3.6.1.2.1.105.1.1.1.3.1.${_port}" i 2
}

function main() {
  local _port="${2}"
  local _action="${1}"
  case "${_action}" in
  'on')
    echo "Powering on device on port ${_port}"
    on "${_port}"
    ;;
  'off')
    echo "Powering off device on port ${_port}"
    off "${_port}"
    ;;
  'reboot')
      echo "Restarting device on port ${_port}"
      off "${_port}"
      sleep 2
      on "${_port}"
    ;;
  'help')
    echo "Usage: $0 [on|off|reboot] [port]"
    ;;
  esac
  exit 0
}

main "$@"