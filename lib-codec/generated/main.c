#include "lib-codec.h"
#include "stdlib.h"
#include "stdio.h"

int main()
{
    CodecResponse resp = Ok;

    printf("Doopie!");

    const struct PyrinasTelemetryData data = {};

    struct Encoded res = encode_telemetry_data(&data);

    printf("res %i", res.resp);

    return 0;
}