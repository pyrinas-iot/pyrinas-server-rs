# Using the example

This document will outline how to use and compile the server and CLI for both local use (for testing) and also production use. We'll start with testing as that's the easier case. 😉

## Setting up for local usage

Make sure you've already built the server. More instructions in the [Readme](../Readme.md#building) You won't be able to use *all* the functionality that Pyrinas has to offer here (OTA and MQTT with remote embedded connections will not work)

### Configuring it

While you should run Pyrinas with encryption turned on, you can run it without for testing purposes. Make sure in your `config.toml` all references to `pkcs12_path` and `pkcs12_pass` are commented out. Also, for clarity, you should modify the port to 1883 rather than 8883.

```toml
[mqtt.rumqtt.servers.1]
port = 1883
#pkcs12_path = "identity.pfx"
#pkcs12_pass = ""
```

### Running the server

If you want to see log output make sure you change the log level first:

```
export RUST_LOG=info
```

Then,

```
./target/release/pyrinas-server config.minimal.toml
```

Note that i'm using the minimal config. This should help you to get chatting with the server side ASAP.

### Running the example client

There also a client that is included in this repository. if you've built with `cargo build --package pyrinas-server --release` the client has already been created! Simply run it like so:

```
> pyrinas-client localhost 1883
```

It will connect to your server and send a measurement every 3 seconds. If logging is on you'll see this on the server side:

```
[2021-03-13T23:39:47Z DEBUG librumqttd::remotelink] data        remote         Id = 12, Count = 1
[2021-03-13T23:39:47Z DEBUG pyrinas_server::mqtt] app: from:"1234"
[2021-03-13T23:39:47Z DEBUG pyrinas_server::broker] broker_run: ApplicationRequest
[2021-03-13T23:39:47Z INFO  pyrinas_server::application] ApplicationRequest(ApplicationData { uid: "1234", target: "env", msg: [162, 107, 116, 101, 109, 112, 101, 114, 97, 116, 117, 114, 101, 25, 15, 160, 104, 104, 117, 109, 105, 100, 105, 116, 121, 25, 12, 128] })
[2021-03-13T23:39:47Z INFO  pyrinas_server::application] EnvironmentData { temperature: 4000, humidity: 3200 }
```

---

## Setting up a production environment

This part of the documentation will show you how to get the example server/client working in a secure production-like environment. We'll go over what needs to be done with the server first and then move over to the embedded client side after that!

If you've already completed some steps or already have a server running feel free to skip the early steps.

### Creating a server

There are a few great resources on this. I run FreeBSD so I typically use [Digital Ocean](https://m.do.co/c/9574d3846a29) or something like [Vultr](https://www.vultr.com/?ref=8484800-6G). (Links help support this project.)

### Building

Make sure you've already built the server. More instructions in the [Readme](../Readme.md#building) 

If you're building on an architecture/OS that is different from the machine you're using now you should either:

* Clone the repo on the server you're going to use
* Use a tool like [`cross` to build it](../Readme.md#using-cross)

### Seting up domain

Generally, you'll want your server available to use directly on the internet. Here's how to configure a subdomain for use with Pyrinas.

Typically you'll need at least one CNAME pointing to the server. 

For example **pyrinas.yourdomain.com**

This is, by far the more (and easy) secure.  I'll get into why in a second.

You should open your domain managmenet webiste and point your new sub-domain to the server/vm/computer you want to host this on. *Make sure this is done first* before configuring Caddy/your reverse proxy of choice.

### Jails/containers

It's *always* a good idea to containerize your server applications to add that extra layer of security in the case that things get compromised. FreeBSD makes it relatively easy due to the great work done on [Bastille](https://bastillebsd.org) (my jail manamagement software of choice).

You can install using `pkg`:

```
>pkg install bastille
```

Then create two servers. One for `caddy` and one for `pyrinas-server`.

```
> bastille bootstrap 12.2-RELEASE
> bastille create caddy 12.2-RELEASE 10.1.1.2
> bastille start caddy
> bastille create pyrinas 12.2-RELEASE 10.1.1.3
> bastille start pyrinas
```

You'll also notice there's a `bootstrap` command. That downloads the base system that get shared by all running containers. You *need* it before continuing on.

### Ports/firewalls

You'll need to open up 3 ports for things to work properly with Pyrinas. I'm using `pf` on FreeBSD. That configuration looks something like:

```
ext_if="vtnet0"
# ! IMPORTANT: this needs to be set before it's copied.
ext_addr=1.2.3.4

# Caddy related
caddy_addr=10.1.1.2
pyrinas_addr=10.1.1.3

set block-policy return
scrub in on $ext_if all fragment reassemble
set skip on lo

# Tables
table <jails> persist
table <badhosts> persist file "/etc/blockaddr"

# Container routes
rdr pass inet proto tcp from any to port 80 -> $caddy_addr port 8880
rdr pass inet proto tcp from any to port 443 -> $caddy_addr port 4443
rdr pass inet proto tcp from any to port 8883 -> $pyrinas_addr port 8883

# Jails
nat on $ext_if from <jails> to any -> $ext_addr

# Blocking ips
block on $ext_if from <badhosts> to any

block in all
pass out quick modulate state
antispoof for $ext_if inet
pass in inet proto tcp from any to any port ssh flags S/SA keep state

# Pass Wireguard
pass in inet proto udp from any to any port 51820
```

You'll want to make sure that `ext_addr` is set to the external address of your server.

Some important notes here:

* `rdr pass inet proto tcp from any to port 80` opens port 80. This helps redirect HTTP traffic to the HTTPs port. 
* `rdr pass inet proto tcp from any to port 443` redirects HTTPS traffic to `caddy` (or similar)
* `rdr pass inet proto tcp from any to port 8883` redirects encrypted MQTT traffic to `pyrinas`

It's important to note that this is dedicating port 8883 on the main server to be used by Pyrinas. So if you are using it for something else you may have to adjust the ports. 

### Creating certs

You can use **easy-rsa** to generate a CA server and client certs. (These instructions come from [this guide](https://github.com/OpenVPN/easy-rsa/blob/master/README.quickstart.md).) For production, you should generate your keys and certs on an offline machine. That way your private keys are safe if your server becomes a target. 

First, install `easy-rsa`:

```bash
$ pkg install easy-rsa
Updating FreeBSD repository catalogue...
FreeBSD repository is up to date.
All repositories are up to date.
The following 1 package(s) will be affected (of 0 checked):

New packages to be INSTALLED:
        easy-rsa: 3.0.7

Number of packages to be installed: 1

44 KiB to be downloaded.

Proceed with this action? [y/N]: y
[pyrinas] [1/1] Fetching easy-rsa-3.0.7.txz: 100%   44 KiB  44.8kB/s    00:01    
Checking integrity... done (0 conflicting)
[pyrinas] [1/1] Installing easy-rsa-3.0.7...
[pyrinas] [1/1] Extracting easy-rsa-3.0.7: 100%
```

Then lets begin the cert creation process!

```
$ easyrsa init-pki

Note: using Easy-RSA configuration from: /usr/local/share/easy-rsa/vars

init-pki complete; you may now create a CA or requests.
Your newly created PKI dir is: /root/pki
$
$ easyrsa build-ca

Note: using Easy-RSA configuration from: /usr/local/share/easy-rsa/vars
Using SSL: openssl OpenSSL 1.1.1d-freebsd  10 Sep 2019

Enter New CA Key Passphrase: 
Re-Enter New CA Key Passphrase: 
Generating RSA private key, 2048 bit long modulus (2 primes)
......................+++++
..................................................................................+++++
e is 65537 (0x010001)
You are about to be asked to enter information that will be incorporated
into your certificate request.
What you are about to enter is what is called a Distinguished Name or a DN.
There are quite a few fields but you can leave some blank
For some fields there will be a default value,
If you enter '.', the field will be left blank.
-----
Common Name (eg: your user, host, or server name) [Easy-RSA CA]:pyrinas.jaredwolff.com

CA creation complete and you may now import and sign cert requests.
Your new CA certificate file for publishing is at:
/root/pki/ca.crt
```

**Note:** You will be prompted for a password at the `build-ca` step. Make sure you keep this password handy. 

Then to generate a server cert use:

```
$ easyrsa gen-req pyrinas nopass

Note: using Easy-RSA configuration from: /usr/local/share/easy-rsa/vars
Using SSL: openssl OpenSSL 1.1.1d-freebsd  10 Sep 2019
Generating a RSA private key
...............+++++
........................................+++++
writing new private key to '/root/pki/easy-rsa-82720.X2NVQ0/tmp.akOxhO'
-----
You are about to be asked to enter information that will be incorporated
into your certificate request.
What you are about to enter is what is called a Distinguished Name or a DN.
There are quite a few fields but you can leave some blank
For some fields there will be a default value,
If you enter '.', the field will be left blank.
-----
Common Name (eg: your user, host, or server name) [pyrinas]:pyrinas.jaredwolff.com

Keypair and certificate request completed. Your files are:
req: /root/pki/reqs/pyrinas.req
key: /root/pki/private/pyrinas.key
$
$ easyrsa sign-req server pyrinas 

Note: using Easy-RSA configuration from: /usr/local/share/easy-rsa/vars
Using SSL: openssl OpenSSL 1.1.1d-freebsd  10 Sep 2019

You are about to sign the following certificate.
Please check over the details shown below for accuracy. Note that this request
has not been cryptographically verified. Please be sure it came from a trusted
source or that you have verified the request checksum with the sender.

Request subject, to be signed as a server certificate for 825 days:

subject=
    commonName                = pyrinas.jaredwolff.com

Type the word 'yes' to continue, or any other input to abort.
  Confirm request details: yes 
Using configuration from /root/pki/easy-rsa-82744.hyuGzt/tmp.lZHLEH
Enter pass phrase for /root/pki/private/ca.key:
Check that the request matches the signature
Signature ok
The Subject's Distinguished Name is as follows
commonName            :ASN.1 12:'pyrinas.jaredwolff.com'
Certificate is to be certified until Nov  3 01:12:53 2022 GMT (825 days)

Write out database with 1 new entries
Data Base Updated

Certificate created at: /root/pki/issued/pyrinas.crt
```

You'll be prompted for both the Common Name (i.e. your server name) and the CA cert password in the above step. **Important**: the **Common Name** needs to match the domain name of your server! (Remember, we wrote that down earlier?)

To generate the nRF9160 cert use:

```
$ easyrsa gen-req nrf9160 nopass batch
$ easyrsa sign-req client nrf9160 batch
```

Follow the same procedure as earlier. The only difference is that we're generating a **client** cert instead of a **server** cert.

Once complete, we'll need some files. Here's a full list below:

**For your pyrinas Server**

- `/root/pki/ca.crt`
- `/root/pki/private/pyrinas.key`
- `/root/pki/issued/pyrinas.crt`

**For your nRF9160 Feather (or similar)**

- `/root/pki/ca.crt`
- `/root/pki/private/nrf9160.key`
- `/root/pki/issued/nrf9160.crt`

### Configuring Caddy

We're going to configure `caddy` next. Login to `caddy`'s console and install the root cert package and Caddy's

```
> bastille console caddy
> pkg install ca_root_nss caddy
```

Then edit/add some entries into your Caddyfile located in `/usr/local/etc/caddy/Caddyfile`. This will cover the OTA download endpoint. 


```
pyrinas.yourdomain.com {
    log
    reverse_proxy 127.0.0.1:3030
    tls {
        ciphers TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA
    }
}
```

Note: the `reverse_proxy` location will differ depending on how you have your server set up.

#### Ciphers

It's important to note the ciphers section. Currently, the nRF9160 only supports a small subset of ciphers including:

* TLS-ECDHE-ECDSA-WITH-AES-256-CBC-SHA384
* TLS-ECDHE-ECDSA-WITH-AES-256-CBC-SHA   
* TLS-ECDHE-ECDSA-WITH-AES-128-CBC-SHA256
* TLS-ECDHE-ECDSA-WITH-AES-128-CBC-SHA  
* TLS-ECDHE-RSA-WITH-AES-256-CBC-SHA         
* TLS-ECDHE-RSA-WITH-AES-128-CBC-SHA256    
* TLS-ECDHE-RSA-WITH-AES-128-CBC-SHA       
* TLS-PSK-WITH-AES-256-CBC-SHA      
* TLS-PSK-WITH-AES-128-CBC-SHA256   
* TLS-PSK-WITH-AES-128-CBC-SHA
* TLS-PSK-WITH-AES-128-CCM-8

Many TLS packages have removed the `CBC` type ciphers since they are not as secure as their `GCM` cousins. Hopefully Noric will address this in  future revisions of their modem firmware for the nRF9160.

In these instructions I will not be setting up an `admin` endpoint at all. This can be accomplished using something like `wireguard` where you're using your `admin` endpoint within a VPN.

Once edited, you should be able to start with:

```
> sysrc caddy_enable="YES"
> service caddy start
```

You can always check the log output at `/var/log/caddy.log` if you run into trouble starting `caddy`.

### Setting up Pyrinas' jail/container

If you haven't already, build and move `pyrinas-server` and `pyrinas-cli` to `/usr/local/bin` in the `pyrinas` container you made earlier. You can also copy your intended `config.toml` to the server as well. When copying I typically copy to the full path using `rsync`/`scp`

```
> scp config.toml user@pyrinas.yourdomain.com:/usr/local/bastille/jails/pyrinas/root/root/
```

Then you can launch `pyrinas` by running:

```
> pyrinas-server /root/config.toml &
```

You can also pipe all output to a log file by running:


```
> export RUST_LOG=info
> pyrinas-server /root/config.toml & > pyrinas.log 2>&1 &
```

That way you can monitor your server instance and run it in the background.

### Building embedded code and provisioning

More about the embedded client side check out the [Pyrinas Zephyr module repository.](https://github.com/pyrinas-iot/pyrinas-zephyr/tree/v1.5.x)

## Other production environments

There are other production environments that you could run Pyrinas in. Docker immediately comes to mind which is popular. While this project doesn't support a Docker example it is open to pull requests to add it!