
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <assert.h>
#include "../chain-core.h"
#include "../chain.h"

void deposit(CroAddressPtr staking)
{
    puts("deposit");
    cro_print_address(staking);
    CroUtxo utxo[5];
    memset(utxo, 0, sizeof(utxo));
    int i;
    
    for (i=0;i<5;i++) {

        char tmp[300];
        memset(tmp, 0, sizeof(tmp));
        sprintf(tmp,"dcro1aj3tv4z40250v9v0aextlsq4pl9qzd7zezd3v6fc392ak00zhtds3d2wyl");
       // memset(utxo[i].address, 0, sizeof(utxo[i].address));
        strcpy(utxo[i].address, tmp);
        int j;
        printf("-------------- %d\n",i);
        printf("\n%s\n", utxo[i].address);        
        sprintf(utxo[i].coin, "%d", i*100);
    }

    cro_deposit(Devnet,staking, "0x2782feb1e457733d83bb738d18b55d91c9b1d7e6", utxo, 5);    
}

void unbond(CroAddressPtr staking)
{
    printf("unbond\n");
    cro_unbond(Devnet,staking, "0x2782feb1e457733d83bb738d18b55d91c9b1d7e6", "1000");
}

void withdraw(CroAddressPtr staking)
{
    printf("withdraw\n");

    const char* viewkeys[2]={"02d1a53beae333dfdd18509a1016c6c0047452c1b8018d21e986e23714d15a4fe7","0286181f61cab62bb901412797e39d59914979801f18ca6b825e5802a803ce6677"};
    cro_withdraw(Devnet,staking, "dcro1aj3tv4z40250v9v0aextlsq4pl9qzd7zezd3v6fc392ak00zhtds3d2wyl", viewkeys, 2);

}
int test_tx() {
    const char* mnemonics= "math original guitar once close news cactus crime cool tank honey file endless neglect catch side cluster clay viable journey october market autumn swing";
    CroHDWalletPtr hdwallet= NULL;
    CroAddressPtr staking= NULL;
    CroAddressPtr transfer= NULL;
    CroAddressPtr viewkey= NULL;
    CroAddressPtr viewkey2= NULL;
    
    cro_restore_hdwallet(mnemonics, &hdwallet);
    cro_create_staking_address(hdwallet, Devnet,&staking,0);    
    cro_create_transfer_address(hdwallet, Devnet,&transfer,0);    
    cro_create_viewkey(hdwallet, Devnet,&viewkey,0);      
    cro_create_viewkey(hdwallet, Devnet,&viewkey2,1);      
    cro_print_address(staking);
    cro_print_address(transfer);
    cro_print_address(viewkey);
    cro_print_address(viewkey2);
    // process
    //deposit(staking);
    //unbond(staking);
    withdraw(staking);

    cro_destroy_address(staking);
    cro_destroy_address(transfer);
    cro_destroy_address(viewkey);
    cro_destroy_address(viewkey2);
    cro_destroy_hdwallet(hdwallet);

    return 0;
}