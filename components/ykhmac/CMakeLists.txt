idf_component_register(
    SRCS "ykhmac.c"
    INCLUDE_DIRS "include"
    REQUIRES aes
    PRIV_REQUIRES sha1
)
