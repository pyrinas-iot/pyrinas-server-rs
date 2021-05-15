const filenames = Deno.args;

interface Certs {
  clientId: string,
  caCert: string;
  privateKey: string,
  clientCert: string
};

// Certs for output
let certs: Certs = { clientId: "", caCert: "", privateKey: "", clientCert: "" };

// Get the contents
certs.clientId = Deno.args[0];
certs.caCert = await Deno.readTextFile("ca.crt");
certs.privateKey = await Deno.readTextFile(Deno.args[0] + ".key");
certs.clientCert = await Deno.readTextFile(Deno.args[0] + ".crt");

let index = certs.clientCert.indexOf("-----BEGIN CERTIFICATE-----\n")
certs.clientCert = certs.clientCert.substr(index);

// Print it
console.log(certs);

// Turn it into a json file
await Deno.writeTextFile(Deno.args[0] + ".certs.json", JSON.stringify(certs));
