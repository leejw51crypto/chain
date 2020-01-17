#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define FAIL 0

#define SUCCESS 0

typedef struct Address Address;

typedef struct HDWallet HDWallet;

typedef HDWallet *HDWalletPtr;

typedef Address *AddressPtr;

/**
 * hdwallet creating using bip44 hdwallet
 */
int32_t cro_create_hdwallet(HDWalletPtr *wallet_out, uint8_t *mnemonics, int32_t mnemonics_length);

/**
 * create staking address from bip44 hdwallet
 */
int32_t cro_create_staking_address(HDWalletPtr wallet_ptr, AddressPtr *address_out, uint32_t index);

/**
 * create utxo address from bip44 wallet, which is for withdrawal, transfer amount
 */
int32_t cro_create_transfer_address(HDWalletPtr wallet_ptr,
                                    AddressPtr *address_out,
                                    uint32_t index);

/**
 * create viewkey, which is for encrypted tx
 */
int32_t cro_create_viewkey(HDWalletPtr wallet_ptr, int32_t _index);

/**
 * destroy address
 */
int32_t cro_destroy_address(AddressPtr addr);

/**
 * destroy bip44 hdwallet
 */
int32_t cro_destroy_hdwallet(HDWalletPtr hdwallet);

/**
 * restore bip44 hdwallet from mnemonics which user gives
 */
int32_t cro_restore_hdwallet(const char *mnemonics_string, HDWalletPtr *wallet_out);

/**
 * print address information
 */
int32_t print_address(AddressPtr address_ptr);
