#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define FAIL -1

#define SUCCESS 0

typedef struct CroAddress CroAddress;

typedef struct CroFee CroFee;

typedef struct CroHDWallet CroHDWallet;

typedef struct CroTx CroTx;

typedef struct CroResult {
  int result;
} CroResult;

typedef CroAddress *CroAddressPtr;

typedef CroFee *CroFeePtr;

typedef CroHDWallet *CroHDWalletPtr;

typedef CroTx *CroTxPtr;

typedef struct CroUtxo {
  uint8_t address[32];
  uint64_t value;
  uint64_t valid_from;
} CroUtxo;

typedef struct CroStakedState {
  uint64_t nonce;
  uint64_t bonded;
  uint64_t unbonded;
  uint64_t unbonded_from;
} CroStakedState;

/**
 * create staking address
 * # Safety
 */
CroResult cro_basic_create_staking_address(CroAddressPtr *address_out);

/**
 * create staking address
 * # Safety
 */
CroResult cro_basic_create_transfer_address(CroAddressPtr *address_out);

/**
 * create viewkey, which is for encrypted tx
 * # Safety
 */
CroResult cro_basic_create_viewkey(CroAddressPtr *address_out);

/**
 * restore staking address
 * 32 bytes
 * # Safety
 */
CroResult cro_basic_restore_staking_address(CroAddressPtr *address_out, const uint8_t *input);

/**
 * restore transfer address
 * 32 bytes
 * # Safety
 */
CroResult cro_basic_restore_transfer_address(CroAddressPtr *address_out, const uint8_t *input);

/**
 * restore viewkey
 * 32 bytes
 * # Safety
 */
CroResult cro_basic_restore_viewkey(CroAddressPtr *address_out, const uint8_t *input);

/**
 * create fee algorithm
 * # Safety
 */
CroResult cro_create_fee_algorithm(CroFeePtr *fee_out,
                                   const char *constant_string,
                                   const char *coeff_string);

/**
 * create hd wallet
 * minimum  300 byte-length is necessary
 * # Safety
 */
CroResult cro_create_hdwallet(CroHDWalletPtr *wallet_out,
                              uint8_t *mnemonics,
                              uint32_t mnemonics_length);

/**
 * create staking address from bip44 hdwallet
 * # Safety
 */
CroResult cro_create_staking_address(CroHDWalletPtr wallet_ptr,
                                     Network network,
                                     CroAddressPtr *address_out,
                                     uint32_t index);

/**
 * create utxo address from bip44 wallet, which is for withdrawal, transfer amount
 * # Safety
 */
CroResult cro_create_transfer_address(CroHDWalletPtr wallet_ptr,
                                      Network network,
                                      CroAddressPtr *address_out,
                                      uint32_t index);

/**
 * create tx
 * # Safety
 */
CroResult cro_create_tx(CroTxPtr *tx_out);

/**
 * create viewkey, which is for encrypted tx
 * # Safety
 */
CroResult cro_create_viewkey(CroHDWalletPtr wallet_ptr,
                             Network network,
                             CroAddressPtr *address_out,
                             uint32_t index);

CroResult cro_deposit(uint8_t network,
                      CroAddressPtr from_ptr,
                      const char *to_address_user,
                      const CroUtxo *utxo,
                      uint32_t utxo_count);

/**
 * destroy address
 * # Safety
 */
CroResult cro_destroy_address(CroAddressPtr addr);

/**
 * destroy fee
 * # Safety
 */
CroResult cro_destroy_fee_algorithm(CroFeePtr fee);

/**
 * destroy bip44 hdwallet
 * # Safety
 */
CroResult cro_destroy_hdwallet(CroHDWalletPtr hdwallet);

/**
 * destroy tx
 * # Safety
 */
CroResult cro_destroy_tx(CroTxPtr tx);

/**
 * estimate fee
 * tx_payload_size: in bytes
 * # Safety
 */
uint64_t cro_estimate_fee(CroFeePtr fee_ptr, uint32_t tx_payload_size);

/**
 * export privatekey as raw bytes
 * 32 bytes
 * # Safety
 */
CroResult cro_export_private(CroAddressPtr address_ptr, uint8_t *dst);

/**
 * extract address as raw bytes
 * minimum 32 length is necessary
 * # Safety
 */
CroResult cro_extract_raw_address(CroAddressPtr address_ptr,
                                  uint8_t *address_output,
                                  uint32_t *address_output_length);

/**
 * get address as string
 * minimum byte length 100 is necessary
 * # Safety
 */
CroResult cro_get_printed_address(CroAddressPtr address_ptr,
                                  uint8_t *address_output,
                                  uint32_t address_output_length);

/**
 * staked -> utxo
 * tendermint_url: ws://localhost:26657/websocket
 */
CroResult cro_get_staked_state(CroAddressPtr from_ptr,
                               const char *tenermint_url_string,
                               CroStakedState *staked_state_user);

/**
 * # Safety
 */
CroResult cro_restore_hdwallet(const char *mnemonics_string, CroHDWalletPtr *wallet_out);

CroResult cro_trasfer(uint8_t network,
                      CroAddressPtr from_ptr,
                      const char *return_address_user,
                      const CroUtxo *spend_utxo,
                      uint32_t spend_utxo_count,
                      const CroUtxo *utxo,
                      uint32_t utxo_count,
                      const char *const *viewkeys,
                      int32_t viewkey_count);

/**
 * add txin
 * txid_string: 64 length hex-char , 32 bytes
 * addr_string: transfer address
 * cro_string: cro unit , example 0.0001
 * # Safety
 */
CroResult cro_tx_add_txin(CroTxPtr tx_ptr,
                          const char *txid_string,
                          uint16_t txindex,
                          const char *addr_string,
                          uint64_t coin);

/**
 * add txin in bytes
 * # Safety
 */
CroResult cro_tx_add_txin_raw(CroTxPtr tx_ptr,
                              uint8_t txid[32],
                              uint16_t txindex,
                              uint8_t addr[32],
                              uint64_t coin);

/**
 * add txout
 * # Safety
 */
CroResult cro_tx_add_txout(CroTxPtr tx_ptr, const char *addr_string, uint64_t coin);

/**
 * add txout with bytes
 * # Safety
 */
CroResult cro_tx_add_txout_raw(CroTxPtr tx_ptr, uint8_t addr[32], uint64_t coin);

/**
 * add viewkey
 * # Safety
 */
CroResult cro_tx_add_viewkey(CroTxPtr tx_ptr, const char *viewkey_string);

/**
 * add viewkey in bytes
 * # Safety
 */
CroResult cro_tx_add_viewkey_raw(CroTxPtr tx_ptr, uint8_t viewkey[33]);

/**
 * extract bytes from singed tx
 * # Safety
 */
CroResult cro_tx_complete_signing(CroTxPtr tx_ptr, uint8_t *output, uint32_t *output_length);

/**
 * prepare tx for signing
 * # Safety
 */
CroResult cro_tx_prepare_for_signing(CroTxPtr tx_ptr, uint8_t network);

/**
 * sign for each txin
 * # Safety
 */
CroResult cro_tx_sign_txin(CroAddressPtr address_ptr, CroTxPtr tx_ptr, uint16_t which_tx_in_user);

/**
 * staked -> staked
 */
CroResult cro_unbond(uint8_t network,
                     CroAddressPtr from_ptr,
                     const char *to_address_user,
                     const char *amount_user);

/**
 * staked -> utxo
 */
CroResult cro_withdraw(uint8_t network,
                       CroAddressPtr from_ptr,
                       const char *_to_user,
                       const char *const *viewkeys,
                       int32_t viewkey_count);
