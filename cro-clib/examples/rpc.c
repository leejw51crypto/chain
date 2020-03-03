
#include <stdio.h>
#include <inttypes.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <assert.h>
#include "../chain-core.h"
#include "../chain.h"

// rate: 0.0 ~ 100.0
int32_t  progress(float rate) 
{
    printf("progress %f\n", rate);
}

void show_wallets()
{
    const int BUFSIZE=1000;
    char buf[BUFSIZE];
     const char* req = "{\"jsonrpc\": \"2.0\", \"method\": \"wallet_list\", \"params\": [], \"id\": 1}";
    CroResult retcode = cro_jsonrpc_call("./.storage", "ws://localhost:26657/websocket", 0xab, req, buf, sizeof(buf), &progress);
    if (retcode.result == 0) {
        printf("response: %s\n", buf);
    } else {
        printf("error: %s\n", buf);
    }
}

void sync()
{
    const int BUFSIZE=1000;
    char buf[BUFSIZE];
    char* name= getenv("CRO_NAME");
    char* passphrase=getenv("CRO_PASSPHRASE");
    char* enckey=getenv("CRO_ENCKEY");
    const char* req_template = "{\"jsonrpc\": \"2.0\", \"method\": \"sync\", \"params\": [{\"name\":\"%s\", \"passphrase\":\"%s\",\"enckey\":\"%s\"}], \"id\": 1}";
    char req[BUFSIZE];
    sprintf(req, req_template, name, passphrase, enckey);

    CroResult retcode = cro_jsonrpc_call("./.storage", "ws://localhost:26657/websocket", 0xab, req, buf, sizeof(buf), &progress);
    if (retcode.result == 0) {
        printf("response: %s\n", buf);
    } else {
        printf("error: %s\n", buf);
    }
}
int test_rpc()
{    
    printf("test rpc\n");
    show_wallets();
    sync();
    return 0;
}

