import hashlib
instruction_name = "global:initialize_permissionless_constant_product_pool_with_config2"
discriminator = hashlib.sha256(instruction_name.encode()).digest()[:8]
byte_array_decimal = list(discriminator)
print(byte_array_decimal)


def get_discriminator(instruction_name):
    return hashlib.sha256(f"global:{instruction_name}".encode()).digest()[:8]

instructions = [
    "swap",
    "add_liquidity",
    "remove_liquidity",
    "create_pool",
    "initialize_permissionless_constant_product_pool_with_config2"
]

for name in instructions:
    disc = get_discriminator(name)
    print(f"{name}: {list(disc)} ({disc.hex()})")
    


from solders.keypair import Keypair
import base58

# Replace with your base58 private key
base58_private_key = '...'  # Your base58-encoded private key
private_key_bytes = base58.b58decode(base58_private_key)
keypair = Keypair.from_bytes(private_key_bytes)
public_key = keypair.pubkey()

print('Public Key:', str(public_key))