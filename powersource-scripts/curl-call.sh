#!/usr/bin/env bash
#set -x

function action_call() {
  local _url="${1}"
  local _uri="${2}"
  curl http://"${_url}"/"${_uri}" >> /dev/null 2>&1
}

function on() {
  local _url="${1}"
  action_call "${_url}" "on"
}

function off() {
  local _url="${1}"
  action_call "${_url}" "reboot"
}

function reboot() {
  local _url="${1}"
  action_call "${_url}" ""
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
    echo "Usage: $0 [TARGET-IP] [on|off|reboot]"
    ;;
  esac
  exit 0
}

main "$@"