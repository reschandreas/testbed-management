#!/usr/bin/env bash
#set -x
readonly IPv4_ADDRESS="10.0.0.158"

function get_url() {
  echo "http://${IPv4_ADDRESS}"
}

function action_call() {
  local _url="${1}"
  local _uri="${2}"
  curl http://"${_url}"/cm\?cmnd="${_uri}" >> /dev/null 2>&1
}

function on() {
  local _url="${1}"
  action_call "${_url}" "Power%20ON"
}

function off() {
  local _url="${1}"
  action_call "${_url}" "Power%20OFF"
}

function main() {
  local _ipv4="${1}"
  local _action="${2}"
  case "${_action}" in
  'on')
    echo "Powering on device"
    on "${_ipv4}"
    ;;
  'off')
    echo "Powering off device"
    off "${_ipv4}"
    ;;
  'reboot')
      echo "Restarting device"
      off "${_ipv4}"
      sleep 1
      on "${_ipv4}"
    ;;
  'help')
    echo "Usage: $0 [SMART-PLUG-IP] [on|off|reboot]"
    ;;
  esac
  exit 0
}

main "$@"