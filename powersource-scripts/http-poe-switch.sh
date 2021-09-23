#!/usr/bin/env bash
#set -x
readonly IPv4_ADDRESS="10.0.0.51"
readonly NO_PASSWORD="779d5edf8cdd943473a9b1507748e9fbb4b543f1a1b212833b07f732768743fb3a17d1ffd91ad42041d49a5a0cd8b7864aaa7b0dbd9029a77dcd8df7d9def8c491c7a3b37137646dffaa8add717b4d381a7632895c76e019540d28844e0b3a997c2c577d8c7ff6263292b42c38071581f626475b3950228579aa884323a23bb028f67ab94be82d7aaf58cc6bd6202ff40a85bd2deee92fe3896f201982dcf8a085037acc416cb5b4be2165bfdbab54063129b2684c4fffdc8f8ff3b0876c606d465e65221f3fd2183f45d865d6fe99b52d7988f57891a3f8c24655d622e613b9ae2d3dc94dba8e4860a82c9b0e2e4ba2d18e73989a1251a9632476b4f47964cd"
readonly PASSWORD="63ae31bcf150964ecf0e3e8fcab511b3c706fe7a13bc1eb917780b17e8f41c36be80fe9ddebcac5e0e1be2979efe30586068d524dc71eedea31cd498d3383e75e60be1b426b7b8ddd425b9b6fee38b1438c54c335cfcb089d1450c4640d7e11949c414bb8141b27c4a2ec442a040d957834847b404ae017be4177c0a2d5aa5c58ffdbc07bdfeba0b2f1338a60c28b4d8cd13eb79d89896adffcb3d07b917c1f901c03fa2ccfa5c99c9541404e0bd5b411b44bf34b37cd7e059d23ea61b2133f344d2195039a7310914de851266a33b45f823ee76cd00cadb18ca68ce5ce17c5a32866fd01ab7d5bd4baed7b88b85c221a2a74e67cdda30206601c2bacace4f14"

function get_url() {
  echo "http://${IPv4_ADDRESS}"
}

function check_login() {
  local _tmp_file="${1}"
  grep "csrftoken: (null)" "${_tmp_file}" >> /dev/null 2>&1
  local _exit="${?}"
  if [ "${_exit}" -eq 0 ];then
    echo -n "1"
  else
    grep "csrftoken:" "${_tmp_file}" >> /dev/null 2>&1
    _exit="${?}"
    if [ "${_exit}" -eq 1 ];then
      echo -n "1"
    fi
    echo -n "0"
  fi
}

function login() {
  local _tmp_file="${1}"
  local _url
  _url=$(get_url)
  curl --location --dump-header "${_tmp_file}" --request GET "${_url}/cs3047e2a2/hpe/config/system.xml?action=login&cred=${NO_PASSWORD}" 2> /dev/null | grep "Bad User or Password" >> /dev/null 2>&1
  local _exit="${?}"
  if [ "${_exit}" -eq 0 ];then
    rm "${_tmp_file}"
    curl --location --dump-header "${_tmp_file}" --request GET "${_url}/cs3047e2a2/hpe/config/system.xml?action=login&cred=${PASSWORD}" >> /dev/null 2>&1
  fi
  check_login "${_tmp_file}"
}

function action_call() {
  local _tmp_file="${1}"
  local _body="${2}"
  local _tmp
  _tmp=$(login "${_tmp_file}")
  if [ "${_tmp}" -eq 0 ];then
    local _url
    _url=$(get_url)
    curl --location -g --request POST "${_url}/cs3047e2a2/hpe/wcd?{SystemGlobalSetting}{TimeRangeList}{PoEPSEUnitList}{PoEStatisticsTable}{DiagnosticsUnitList}{LocateUnit}" \
      -H @"${_tmp_file}" \
      --header 'Content-Type: application/xml' \
      --data-raw "${_body}" >> /dev/null 2>&1
  else
    echo "could not login"
  fi
}

function on() {
  local _tmp_file="${1}"
  local _port="${2}"
  action_call "${_tmp_file}" "<?xml version: '1.0' encoding='utf-8'?>
                <DeviceConfiguration>
                    <PoEPSEInterfaceList action=\"set\">
                        <Interface>
                            <interfaceName>${_port}</interfaceName>
                            <interfaceID>${_port}</interfaceID>
                            <adminEnable>1</adminEnable>
                            <timeRangeName></timeRangeName>
                            <powerPriority>3</powerPriority>
                            <portLegacy_powerDetectType>2</portLegacy_powerDetectType>
                            <powerManagementMode>1</powerManagementMode>
                            <highPowerMode>2</highPowerMode>
                        </Interface>
                    </PoEPSEInterfaceList>
                </DeviceConfiguration>"
}

function off() {
  local _tmp_file="${1}"
  local _port="${2}"
  action_call "${_tmp_file}" "<?xml version: '1.0' encoding='utf-8'?>
                <DeviceConfiguration>
                    <PoEPSEInterfaceList action=\"set\">
                        <Interface>
                            <interfaceName>${_port}</interfaceName>
                            <interfaceID>${_port}</interfaceID>
                            <adminEnable>2</adminEnable>
                            <timeRangeName></timeRangeName>
                            <powerPriority>3</powerPriority>
                            <portLegacy_powerDetectType>2</portLegacy_powerDetectType>
                            <powerManagementMode>1</powerManagementMode>
                            <highPowerMode>2</highPowerMode>
                        </Interface>
                    </PoEPSEInterfaceList>
                </DeviceConfiguration>"
}

function main() {
  local _port="${2}"
  local _action="${1}"
  local _tmp_dir
  _tmp_dir=$(mktemp -d -t ci-XXXXXXXXXX)
  local _tmp_file="${_tmp_dir}/curl_headers"
  touch "${_tmp_file}"
  case "${_action}" in
  'on')
    echo "Powering on device on port ${_port}"
    on "${_tmp_file}" "${_port}"
    ;;
  'off')
    echo "Powering off device on port ${_port}"
    off "${_tmp_file}" "${_port}"
    ;;
  'reboot')
      echo "Restarting device on port ${_port}"
      off "${_tmp_file}" "${_port}"
      sleep 2
      on "${_tmp_file}" "${_port}"
    ;;
  'help')
    echo "Usage: $0 [on|off|reboot] [port]"
    ;;
  esac
  rm -rf "${_tmp_dir}"
  exit 0
}

main "$@"