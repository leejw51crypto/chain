#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include "../chain.h"

int main() {
    HDWalletPtr w=NULL;
  
    char tmp[300];
    memset(tmp, 0, 300);
    cro_create_hdwallet(&w, tmp, 300);
    printf("mnemonic=%s\n",tmp);
    AddressPtr a= NULL;
    cro_create_staking_address(w, &a,0);
    print_address(a);   
    cro_destroy_address(a);  
    cro_create_transfer_address(w, &a, 0);
    print_address(a);   
    cro_destroy_address(a);
    cro_destroy_hdwallet(w);
    HDWalletPtr q= NULL;
    cro_restore_hdwallet(tmp, &q);
    cro_create_staking_address(q, &a,0);
    print_address(a);
    cro_destroy_address(a);
    cro_create_transfer_address(w, &a,0);  
    print_address(a);   
    cro_destroy_address(a);
    cro_destroy_hdwallet(q);
    return 0;
}
