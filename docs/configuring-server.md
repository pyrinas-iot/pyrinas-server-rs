# Configuring Pyrinas Server

There are some important pieces to take care of in order to fully utilize Pyrinas. See below for each item.

## Hosting

- [ ] Self hosting option
- [ ] Hosting in a container

## Opening up firewall

* 443 - for https
* 8883 - mqtt

## Generating certs

- [ ] Provision certs 

### Generating server identity file

`openssl` generate `.p12`

### Installing onto nRF9160 Feather

Manifest blah.

## Using with Caddy

Currrently, Pyrinas Server has been tested with Caddy as a reverse proxy. Below are some examples of how you can configure external facing endpoints: `ota` and `admin`.


```
ota.yourdomain.com {
    log
    reverse_proxy 127.0.0.1:3030
    tls {
        ciphers TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA TLS_ECDHE_RSA_WITH_AES_256_CBC_SHA
    }
}

admin.yourdomain.com {
    log
    reverse_proxy 127.0.0.1:3032
    encode gzip
}
```

Note: the `reverse_proxy` location will differ depending on how you have your server set up. It's always recommended to run Pyrinas in a protected jail or container.

### Ciphers

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

Another suggestion is not to expose your `admin` endpoint at all. This can be accomplished using something like `wireguard` where you're using your `admin` endpoint within a VPN.
