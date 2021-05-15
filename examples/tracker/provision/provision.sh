#!/usr/bin/env bash

# Enter your PKI server address
server_string="root@1.2.3.4"

# PKI Path setup by EasyRSA
pki_path="/root/pki"

# Provisions new hub on Dreamstar Cloud
main() {
  local device_id="${1}"

  # Invoke these guys
  if [ ! -f "${1}.certs.json" ]; then
    echo "Generating certificate."
    ssh ${server_string} bastille cmd ${jail_name} "easyrsa gen-req ${1} nopass batch"
    ssh ${server_string} bastille cmd ${jail_name} "easyrsa sign-req client ${1} batch"

    # Then, fetch the results
    scp ${server_string}:"${pki_path}/ca.crt" .
    scp ${server_string}:"${pki_path}/private/${1}.key" .
    scp ${server_string}:"${pki_path}/issued/${1}.crt" .

    # Then place into JSON template
    deno run --allow-write --allow-read  certify.ts ${1}

    # Cleanup
    rm ca.crt
    rm ${1}.key
    rm ${1}.crt

  else
    echo "Cert for ${1} already exists!"
  fi

}

main "${@}"