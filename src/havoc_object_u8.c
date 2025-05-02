unsigned char __cbmc_havoc_object_u8(unsigned char *ptr, unsigned long len)
{
  __CPROVER_assert(ptr != 0, "pointer is not NULL");
  __CPROVER_havoc_slice(ptr, len);
  return 0;
}
