# Osprey render CPU sample

Generated mechanically with macOS sample from the current symbol-enabled release binary, during a full cold scan of the pinned Osprey checkout.

    Analysis of sampling deslop (pid 22976) every 10 milliseconds
    Process:         deslop [22976]
    Path:            /Users/USER/Documents/*/deslop
    Load Address:    0x102e04000
    Identifier:      deslop
    Version:         0
    Code Type:       ARM64
    Platform:        macOS
    Parent Process:  Python [22975]
    Target Type:     live task
    
    Date/Time:       2026-11-08 15:39:10.885 +1100
    Launch Time:     2026-11-08 15:38:11.895 +1100
    OS Version:      macOS 26.7.1 (25G241)
    Report Version:  7
    Analysis Tool:   /usr/bin/sample
    
    Physical footprint:         8.5G
    Physical footprint (peak):  14.5G
    Idle exit:                  untracked
    ----
    
    Call graph:
        356 Thread_5540787   DispatchQueue_1: com.apple.main-thread  (serial)
          356 start  (in dyld) + 6992  [0x1852c84e4]
            356 main  (in deslop) + 52  [0x102eebba8]
              356 _RNvNtCsdnanKjkF2cC_3std2rt19lang_start_internal  (in deslop) + 936  [0x103164a80]  rt.rs:171
                356 _RNCINvNtCsdnanKjkF2cC_3std2rt10lang_startuE0Csjygw1zI1mNd_6deslop  (in deslop) + 16  [0x102eafff4]  rt.rs:206
                  356 _RINvNtNtCsdnanKjkF2cC_3std3sys9backtrace28___rust_begin_short_backtraceFEuuECsjygw1zI1mNd_6deslop  (in deslop) + 12  [0x102e7926c]  backtrace.rs:166
                    356 _RNvCsjygw1zI1mNd_6deslop4main  (in deslop) + 8356  [0x102eb4768]  main.rs:267
                      356 _RNCNvCsjygw1zI1mNd_6deslop7run_cli0B3_  (in deslop) + 740  [0x102eb02e4]  main.rs:331
                        356 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session10initialiseNtB4_15PipelineSession23initialise_from_sources  (in deslop) + 8304  [0x102fabae8]
                          356 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6renderNtB4_15PipelineSession6render  (in deslop) + 4792  [0x102fb5378]  render.rs:110
                            354 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6renderNtB4_15PipelineSession19partition_and_split  (in deslop) + 728  [0x102fb065c]  render.rs:184
                            + 257 _RNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information22family_recovery_groups  (in deslop) + 128  [0x103044154]  information.rs:210
                            + ! 257 _RNvXs0_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB5_19ClusterKindMeasurerNtNtBd_7cluster16ClusterKindJudge6groups  (in deslop) + 520  [0x1030553c8]  cluster_kind.rs:359
                            + !   256 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer9pair_kind  (in deslop) + 456  [0x102fd595c]  cluster_kind.rs:203
                            + !   : 252 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 1204  [0x102fd5308]  cluster_kind.rs:225
                            + !   : | 108 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 740  [0x102fae0a4]  pair_compare.rs:116
                            + !   : | + 31 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            + !   : | + ! 22 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + !   : | + ! : 12 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + !   : | + ! : | 7 _platform_memcmp  (in libsystem_platform.dylib) + 128,0,...  [0x18569af90,0x18569af10,...]
                            + !   : | + ! : | 5 DYLD-STUB$$memcmp  (in deslop) + 4  [0x10329cd9c]
                            + !   : | + ! : 3 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 756  [0x10303fef4]  literal_echo.rs:164
                            + !   : | + ! : | 3 _platform_memmove  (in libsystem_platform.dylib) + 452,428  [0x18569d524,0x18569d50c]
                            + !   : | + ! : 3 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 836  [0x10303ff44]  literal_echo.rs:166
                            + !   : | + ! : | 2 _xzm_free  (in libsystem_malloc.dylib) + 220,996  [0x1854b01f4,0x1854b04fc]
                            + !   : | + ! : | 1 _free  (in libsystem_malloc.dylib) + 0  [0x1854bfcd0]
                            + !   : | + ! : 2 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172,196  [0x10303fcac,0x10303fcc4]  literal_echo.rs:154
                            + !   : | + ! : 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 108  [0x10303fc6c]  literal_echo.rs:151
                            + !   : | + ! : 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 924  [0x10303ff9c]  literal_echo.rs:164
                            + !   : | + ! :   1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + ! :     1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + ! :       1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + ! :         1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | + ! :           1 xzm_realloc  (in libsystem_malloc.dylib) + 24  [0x1854afb9c]
                            + !   : | + ! 4 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 116  [0x102f1f170]  literal_echo.rs:41
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 360  [0x102fa9248]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + !   : | + ! : |   1 _xzm_free  (in libsystem_malloc.dylib) + 684  [0x1854b03c4]
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 128  [0x102fa9160]  corroboration.rs:26
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1188  [0x102fa9584]  corroboration.rs:27
                            + !   : | + ! : | 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + !   : | + ! : |   1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + !   : | + ! : |     1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1940  [0x102fa9874]  corroboration.rs:27
                            + !   : | + ! 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 88  [0x102f1f154]  literal_echo.rs:40
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10population  (in deslop) + 228  [0x103025920]  frontier.rs:139
                            + !   : | + ! 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 100  [0x102f1f160]  literal_echo.rs:41
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17substituted_pairs  (in deslop) + 188  [0x1030257d8]  rename.rs:374
                            + !   : | + ! :   1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + ! :     1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + ! :       1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + ! :         1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | + ! :           1 xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
                            + !   : | + ! :             1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 256  [0x1854b503c]
                            + !   : | + ! 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 680  [0x102f1f3a4]  literal_echo.rs:42
                            + !   : | + ! 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1012  [0x102f1f4f0]  literal_echo.rs:44
                            + !   : | + ! : 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo29substituted_literal_positions  (in deslop) + 100  [0x103040038]  literal_echo.rs:97
                            + !   : | + ! :   1 <deduplicated_symbol>  (in libsystem_malloc.dylib) + 112  [0x1854af1cc]
                            + !   : | + ! 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1496  [0x102f1f6d4]  literal_echo.rs:59
                            + !   : | + !   1 _xzm_free  (in libsystem_malloc.dylib) + 120  [0x1854b0190]
                            + !   : | + 29 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
                            + !   : | + ! 22 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + !   : | + ! : 13 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + !   : | + ! : | 9 _platform_memcmp  (in libsystem_platform.dylib) + 0,236,...  [0x18569af10,0x18569affc,...]
                            + !   : | + ! : | 4 DYLD-STUB$$memcmp  (in deslop) + 4  [0x10329cd9c]
                            + !   : | + ! : 3 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 72  [0x10303fc48]  literal_echo.rs:151
                            + !   : | + ! : | 3 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 96,56  [0x1854b4f9c,0x1854b4f74]
                            + !   : | + ! : 3 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 176  [0x10303fcb0]  literal_echo.rs:154
                            + !   : | + ! : 2 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 836  [0x10303ff44]  literal_echo.rs:166
                            + !   : | + ! : | 1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + !   : | + ! : | + 1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + !   : | + ! : | 1 _xzm_free  (in libsystem_malloc.dylib) + 684  [0x1854b03c4]
                            + !   : | + ! : 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 756  [0x10303fef4]  literal_echo.rs:164
                            + !   : | + ! :   1 _platform_memmove  (in libsystem_platform.dylib) + 0  [0x18569d360]
                            + !   : | + ! 6 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 116  [0x102f1f170]  literal_echo.rs:41
                            + !   : | + ! : 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 764  [0x102fa93dc]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + !   : | + ! : | + 1 _xzm_free  (in libsystem_malloc.dylib) + 352  [0x1854b0278]
                            + !   : | + ! : | +   1 _platform_memset  (in libsystem_platform.dylib) + 144  [0x18569d120]
                            + !   : | + ! : | 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 60  [0x102fdadc0]  map.rs:1940
                            + !   : | + ! : 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 996,1428  [0x102fa94c4,0x102fa9674]  corroboration.rs:27
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 244  [0x102fa91d4]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 116  [0x102fc1074]  entry.rs:147
                            + !   : | + ! : |   1 _malloc_zone_malloc  (in libsystem_malloc.dylib) + 12  [0x1854bf0c8]
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1188  [0x102fa9584]  corroboration.rs:27
                            + !   : | + ! :   1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + !   : | + ! :     1 _xzm_free  (in libsystem_malloc.dylib) + 320  [0x1854b0258]
                            + !   : | + ! 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1476  [0x102f1f6c0]  literal_echo.rs:59
                            + !   : | + !   1 _free  (in libsystem_malloc.dylib) + 0  [0x1854bfcd0]
                            + !   : | + 10 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9668  [0x102f0e984]  content.rs:264
                            + !   : | + ! 8 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + !   : | + ! : 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 400,416  [0x102fa9270,0x102fa9280]  corroboration.rs:26
                            + !   : | + ! : 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1072  [0x102fa9510]  corroboration.rs:27
                            + !   : | + ! : | 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 116  [0x102fc1074]  entry.rs:147
                            + !   : | + ! : | + 1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 256  [0x1854b503c]
                            + !   : | + ! : | 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 44  [0x102fc102c]  entry.rs:147
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 244  [0x102fa91d4]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 440  [0x102fc11b8]  entry.rs:147
                            + !   : | + ! : |   1 _platform_memmove  (in libsystem_platform.dylib) + 464  [0x18569d530]
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 540  [0x102fa92fc]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryyTyjEE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 132  [0x102fc1784]  entry.rs:147
                            + !   : | + ! : |   1 <deduplicated_symbol>  (in libsystem_malloc.dylib) + 112  [0x1854af1cc]
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 692  [0x102fa9394]  corroboration.rs:26
                            + !   : | + ! : | 1 <deduplicated_symbol>  (in libsystem_malloc.dylib) + 136  [0x1854af1e4]
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1684  [0x102fa9774]  corroboration.rs:27
                            + !   : | + ! :   1 _RINvMsi_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB6_8BTreeMapyyE27bulk_build_from_sorted_iterINtNtBc_3vec3VecTyyEEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 688  [0x102ef5c50]  map.rs:1716
                            + !   : | + ! :     1 DYLD-STUB$$free  (in deslop) + 4  [0x10329cc94]
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 92  [0x103024e2c]  rename.rs:331
                            + !   : | + ! : 1 _RINvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename11pair_countsINtNtNtNtCs3bJEMBXHbwl_4core4iter8adapters6copied6CopiedINtNtNtB18_5slice4iter4IterTyyEEEEB6_  (in deslop) + 224  [0x102f16cc8]  rename.rs:436
                            + !   : | + ! :   1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 464  [0x102fc11d0]  entry.rs:147
                            + !   : | + ! :     1 DYLD-STUB$$memmove  (in deslop) + 4  [0x10329cdb4]
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 272  [0x103024ee0]  rename.rs:341
                            + !   : | + 8 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 296  [0x102f0c4e8]  content.rs:257
                            + !   : | + ! 3 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 360  [0x102fa9248]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 228  [0x102fdae68]  map.rs:1940
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 108  [0x102fa914c]  corroboration.rs:26
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 996  [0x102fa94c4]  corroboration.rs:27
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 56  [0x103024e08]  rename.rs:329
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17substituted_pairs  (in deslop) + 188  [0x1030257d8]  rename.rs:374
                            + !   : | + ! :   1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + ! :     1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + ! :       1 _realloc  (in libsystem_malloc.dylib) + 36  [0x1854bfd74]
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 92  [0x103024e2c]  rename.rs:331
                            + !   : | + ! : 1 _RINvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename11pair_countsINtNtNtNtCs3bJEMBXHbwl_4core4iter8adapters6copied6CopiedINtNtNtB18_5slice4iter4IterTyyEEEEB6_  (in deslop) + 224  [0x102f16cc8]  rename.rs:436
                            + !   : | + ! :   1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 464  [0x102fc11d0]  entry.rs:147
                            + !   : | + ! :     1 _platform_memmove  (in libsystem_platform.dylib) + 676  [0x18569d604]
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 452  [0x103024f94]  rename.rs:341
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 1668  [0x103025454]  rename.rs:353
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 1100  [0x10302521c]  rename.rs:354
                            + !   : | + 7 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 80  [0x102f0c410]  content.rs:254
                            + !   : | + ! 6 _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 76  [0x102f1040c]  content.rs:315
                            + !   : | + ! : 5 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 172  [0x103023c48]  call_targets.rs:292
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 244  [0x102fa91d4]  corroboration.rs:26
                            + !   : | + ! : | + 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 116  [0x102fc1074]  entry.rs:147
                            + !   : | + ! : | +   1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 284  [0x1854b5058]
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 360  [0x102fa9248]  corroboration.rs:26
                            + !   : | + ! : | + 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + !   : | + ! : | +   1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + !   : | + ! : | +     1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 692  [0x102fa9394]  corroboration.rs:26
                            + !   : | + ! : | + 1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 824  [0x1854b5274]
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 416  [0x102fa9280]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1968  [0x102fa9890]  corroboration.rs:27
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 1656  [0x103024214]  call_targets.rs:292
                            + !   : | + ! :   1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 236  [0x102fc10ec]  entry.rs:147
                            + !   : | + ! :     1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 168  [0x1854b4fe4]
                            + !   : | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 48  [0x102f103f0]  content.rs:312
                            + !   : | + !   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier22operator_contradiction  (in deslop) + 116  [0x103025a64]  frontier.rs:185
                            + !   : | + 6 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 336  [0x102f0c510]  content.rs:257
                            + !   : | + ! 3 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 708  [0x102fa1778]  rename.rs:182
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 856  [0x102fa9438]  corroboration.rs:26
                            + !   : | + ! : | 1 _RINvMsi_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB6_8BTreeMapyyE27bulk_build_from_sorted_iterINtNtBc_3vec3VecTyyEEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 284  [0x102ef5abc]  map.rs:1716
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1072  [0x102fa9510]  corroboration.rs:27
                            + !   : | + ! : | 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 464  [0x102fc11d0]  entry.rs:147
                            + !   : | + ! : |   1 _platform_memmove  (in libsystem_platform.dylib) + 464  [0x18569d530]
                            + !   : | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1288  [0x102fa95e8]  corroboration.rs:27
                            + !   : | + ! 1 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 656  [0x102fa1744]  rename.rs:181
                            + !   : | + ! 1 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 692  [0x102fa1768]  rename.rs:182
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17substituted_pairs  (in deslop) + 96  [0x10302577c]  rename.rs:374
                            + !   : | + ! :   1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 684  [0x1854b51e8]
                            + !   : | + ! 1 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 1276  [0x102fa19b0]  rename.rs:196
                            + !   : | + 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 480  [0x102f0c5a0]  content.rs:264
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17literal_positions  (in deslop) + 328  [0x103025694]  rename.rs:241
                            + !   : | + ! : 1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + ! :   1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + ! :     1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + ! :       1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | + ! :         1 xzm_realloc  (in libsystem_malloc.dylib) + 904  [0x1854aff0c]
                            + !   : | + ! :           1 _xzm_free  (in libsystem_malloc.dylib) + 28  [0x1854b0134]
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17literal_positions  (in deslop) + 240  [0x10302563c]  rename.rs:241
                            + !   : | + 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 548  [0x102f0c5e4]  content.rs:264
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10population  (in deslop) + 152  [0x1030258d4]  frontier.rs:139
                            + !   : | + ! : 1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 420  [0x1854b50e0]
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10population  (in deslop) + 216  [0x103025914]  frontier.rs:139
                            + !   : | + 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 2392  [0x102f0cd18]  content.rs:264
                            + !   : | + ! 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename10consistent20strip_common_affixes  (in deslop) + 228  [0x10303fb40]  consistent.rs:169
                            + !   : | + ! : 1 _xzm_xzone_malloc  (in libsystem_malloc.dylib) + 192  [0x1854afa10]
                            + !   : | + ! 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename10consistent20strip_common_affixes  (in deslop) + 316  [0x10303fb98]  consistent.rs:169
                            + !   : | + !   1 _platform_memmove  (in libsystem_platform.dylib) + 460  [0x18569d52c]
                            + !   : | + 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 5024  [0x102f0d760]  content.rs:264
                            + !   : | + ! 2 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 268,420  [0x1854b5048,0x1854b50e0]
                            + !   : | + 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 7780  [0x102f0e224]  content.rs:264
                            + !   : | + ! 2 _platform_memcmp  (in libsystem_platform.dylib) + 168,272  [0x18569afb8,0x18569b020]
                            + !   : | + 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 8000  [0x102f0e300]  content.rs:264
                            + !   : | + ! 2 _free  (in libsystem_malloc.dylib) + 0,36  [0x1854bfcd0,0x1854bfcf4]
                            + !   : | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 208  [0x102f0c490]  content.rs:257
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17literal_positions  (in deslop) + 152  [0x1030255e4]  rename.rs:241
                            + !   : | + !   1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 420  [0x1854b50e0]
                            + !   : | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 2360  [0x102f0ccf8]  content.rs:264
                            + !   : | + ! 1 _RINvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10leaf_bytesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB6_  (in deslop) + 68  [0x102f16d78]  frontier.rs:123
                            + !   : | + !   1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRNtNtCs7J0RnZC7BVr_11deslop_core5state6FileIdEB1J_  (in deslop) + 264  [0x102f865e8]  mod.rs:702
                            + !   : | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 2772  [0x102f0ce94]  content.rs:264
                            + !   : | + ! 1 _platform_memmove  (in libsystem_platform.dylib) + 0  [0x18569d360]
                            + !   : | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 7652  [0x102f0e1a4]  content.rs:264
                            + !   : | + ! 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyETINtNtBb_3vec3VechEB1c_EE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdad48]  map.rs:1936
                            + !   : | + !   1 _xzm_free  (in libsystem_malloc.dylib) + 352  [0x1854b0278]
                            + !   : | + !     1 _platform_memset  (in libsystem_platform.dylib) + 140  [0x18569d11c]
                            + !   : | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9732  [0x102f0e9c4]  content.rs:264
                            + !   : | +   1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 24  [0x102fdad9c]  map.rs:1935
                            + !   : | 64 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1812  [0x102fae4d4]  pair_compare.rs:153
                            + !   : | + 43 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 228  [0x102ef2434]  cache.rs:139
                            + !   : | + ! 10 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            + !   : | + ! : 6 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + !   : | + ! : | 3 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 924  [0x10303ff9c]  literal_echo.rs:164
                            + !   : | + ! : | + 3 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + ! : | +   3 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + ! : | +     2 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + ! : | +     ! 2 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | + ! : | +     !   1 xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
                            + !   : | + ! : | +     !   : 1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 24  [0x1854b4f54]
                            + !   : | + ! : | +     !   1 xzm_realloc  (in libsystem_malloc.dylib) + 264  [0x1854afc8c]
                            + !   : | + ! : | +     1 _realloc  (in libsystem_malloc.dylib) + 252  [0x1854bfe4c]
                            + !   : | + ! : | +       1 xzm_malloc_zone_size  (in libsystem_malloc.dylib) + 164  [0x1854ba5b0]
                            + !   : | + ! : | 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 108  [0x10303fc6c]  literal_echo.rs:151
                            + !   : | + ! : | 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + !   : | + ! : | + 1 DYLD-STUB$$memcmp  (in deslop) + 4  [0x10329cd9c]
                            + !   : | + ! : | 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 176  [0x10303fcb0]  literal_echo.rs:154
                            + !   : | + ! : 4 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 116  [0x102f1f170]  literal_echo.rs:41
                            + !   : | + ! :   2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 76,764  [0x102fa912c,0x102fa93dc]  corroboration.rs:26
                            + !   : | + ! :   1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 244  [0x102fa91d4]  corroboration.rs:26
                            + !   : | + ! :   + 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 464  [0x102fc11d0]  entry.rs:147
                            + !   : | + ! :   +   1 _platform_memmove  (in libsystem_platform.dylib) + 0  [0x18569d360]
                            + !   : | + ! :   1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 360  [0x102fa9248]  corroboration.rs:26
                            + !   : | + ! :     1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + !   : | + ! :       1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + !   : | + ! :         1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + !   : | + ! 6 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
                            + !   : | + ! : 4 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + !   : | + ! : | 3 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + !   : | + ! : | + 2 _platform_memcmp  (in libsystem_platform.dylib) + 48,140  [0x18569af40,0x18569af9c]
                            + !   : | + ! : | + 1 DYLD-STUB$$memcmp  (in deslop) + 4  [0x10329cd9c]
                            + !   : | + ! : | 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 836  [0x10303ff44]  literal_echo.rs:166
                            + !   : | + ! : |   1 _xzm_free  (in libsystem_malloc.dylib) + 996  [0x1854b04fc]
                            + !   : | + ! : 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 116  [0x102f1f170]  literal_echo.rs:41
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1188  [0x102fa9584]  corroboration.rs:27
                            + !   : | + ! : |   1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + !   : | + ! : |     1 _xzm_free  (in libsystem_malloc.dylib) + 684  [0x1854b03c4]
                            + !   : | + ! : 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 848  [0x102f1f44c]  literal_echo.rs:42
                            + !   : | + ! :   1 _RINvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10leaf_bytesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB6_  (in deslop) + 68  [0x102f16d78]  frontier.rs:123
                            + !   : | + ! :     1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRNtNtCs7J0RnZC7BVr_11deslop_core5state6FileIdEB1J_  (in deslop) + 264  [0x102f865e8]  mod.rs:702
                            + !   : | + ! 6 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9668  [0x102f0e984]  content.rs:264
                            + !   : | + ! : 3 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + !   : | + ! : | 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 128,416  [0x102fa9160,0x102fa9280]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1244  [0x102fa95bc]  corroboration.rs:27
                            + !   : | + ! : 2 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 1232  [0x1030252a0]  rename.rs:350
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 56  [0x103024e08]  rename.rs:329
                            + !   : | + ! :   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17substituted_pairs  (in deslop) + 188  [0x1030257d8]  rename.rs:374
                            + !   : | + ! :     1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + ! :       1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 44  [0x10327f8b8]
                            + !   : | + ! 5 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 336  [0x102f0c510]  content.rs:257
                            + !   : | + ! : 4 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 708  [0x102fa1778]  rename.rs:182
                            + !   : | + ! : | 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 344,1800  [0x102fa9238,0x102fa97e8]  corroboration.rs:26
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 856  [0x102fa9438]  corroboration.rs:26
                            + !   : | + ! : | + 1 _RINvMsi_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB6_8BTreeMapyyE27bulk_build_from_sorted_iterINtNtBc_3vec3VecTyyEEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 688  [0x102ef5c50]  map.rs:1716
                            + !   : | + ! : | +   1 _xzm_free  (in libsystem_malloc.dylib) + 684  [0x1854b03c4]
                            + !   : | + ! : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1592  [0x102fa9718]  corroboration.rs:27
                            + !   : | + ! : |   1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 124  [0x102fdae00]  map.rs:1936
                            + !   : | + ! : 1 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 740  [0x102fa1798]  rename.rs:183
                            + !   : | + ! :   1 _RINvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename11pair_countsINtNtNtNtCs3bJEMBXHbwl_4core4iter8adapters6copied6CopiedINtNtNtB18_5slice4iter4IterTyyEEEEB6_  (in deslop) + 68  [0x102f16c2c]  rename.rs:436
                            + !   : | + ! 4 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 276  [0x102f0c4d4]  content.rs:257
                            + !   : | + ! : 3 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10population  (in deslop) + 312  [0x103025974]  frontier.rs:139
                            + !   : | + ! : | 3 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + ! : |   3 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + ! : |     3 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + ! : |       3 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | + ! : |         1 xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
                            + !   : | + ! : |         + 1 <deduplicated_symbol>  (in libsystem_malloc.dylib) + 112  [0x1854af1cc]
                            + !   : | + ! : |         1 xzm_realloc  (in libsystem_malloc.dylib) + 904  [0x1854aff0c]
                            + !   : | + ! : |         + 1 _xzm_free  (in libsystem_malloc.dylib) + 164  [0x1854b01bc]
                            + !   : | + ! : |         1 xzm_realloc  (in libsystem_malloc.dylib) + 236  [0x1854afc70]
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10population  (in deslop) + 152  [0x1030258d4]  frontier.rs:139
                            + !   : | + ! :   1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 12  [0x1854b4f48]
                            + !   : | + ! 3 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 296  [0x102f0c4e8]  content.rs:257
                            + !   : | + ! : 2 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + !   : | + ! : | 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 360  [0x102fa9248]  corroboration.rs:26
                            + !   : | + ! : |   2 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + !   : | + ! : |     1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + !   : | + ! : |     + 1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + !   : | + ! : |     1 _xzm_free  (in libsystem_malloc.dylib) + 256  [0x1854b0218]
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 820  [0x103025104]  rename.rs:350
                            + !   : | + ! 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 80  [0x102f0c410]  content.rs:254
                            + !   : | + ! : 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 76  [0x102f1040c]  content.rs:315
                            + !   : | + ! :   2 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 172  [0x103023c48]  call_targets.rs:292
                            + !   : | + ! :     1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 244  [0x102fa91d4]  corroboration.rs:26
                            + !   : | + ! :     | 1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 32  [0x102fc1020]  entry.rs:145
                            + !   : | + ! :     1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 856  [0x102fa9438]  corroboration.rs:26
                            + !   : | + ! :       1 _RINvMsi_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB6_8BTreeMapyyE27bulk_build_from_sorted_iterINtNtBc_3vec3VecTyyEEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 256  [0x102ef5aa0]  map.rs:1716
                            + !   : | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 548  [0x102f0c5e4]  content.rs:264
                            + !   : | + ! : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10population  (in deslop) + 152  [0x1030258d4]  frontier.rs:139
                            + !   : | + ! :   1 _xzm_xzone_malloc  (in libsystem_malloc.dylib) + 180  [0x1854afa04]
                            + !   : | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 4980  [0x102f0d734]  content.rs:264
                            + !   : | + ! : 1 _malloc_zone_malloc  (in libsystem_malloc.dylib) + 120  [0x1854bf134]
                            + !   : | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 5000  [0x102f0d748]  content.rs:264
                            + !   : | + ! : 1 _platform_memmove  (in libsystem_platform.dylib) + 436  [0x18569d514]
                            + !   : | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 5044  [0x102f0d774]  content.rs:264
                            + !   : | + ! : 1 _platform_memmove  (in libsystem_platform.dylib) + 12  [0x18569d36c]
                            + !   : | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 7652  [0x102f0e1a4]  content.rs:264
                            + !   : | + ! : 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyETINtNtBb_3vec3VechEB1c_EE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 24  [0x102fdac18]  map.rs:1935
                            + !   : | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9332  [0x102f0e834]  content.rs:264
                            + !   : | + ! : 1 _xzm_free  (in libsystem_malloc.dylib) + 196  [0x1854b01dc]
                            + !   : | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 10568  [0x102f0ed08]  content.rs:264
                            + !   : | + !   1 _xzm_free  (in libsystem_malloc.dylib) + 48  [0x1854b0148]
                            + !   : | + 13 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 196  [0x102ef2414]  cache.rs:139
                            + !   : | + ! 4 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 176  [0x1030248f0]  core_frontier.rs:148
                            + !   : | + ! : 2 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent5slice  (in deslop) + 428  [0x102fcf698]  core_frontier.rs:58
                            + !   : | + ! : | 2 _xzm_xzone_malloc  (in libsystem_malloc.dylib) + 8,180  [0x1854af958,0x1854afa04]
                            + !   : | + ! : 1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent5slice  (in deslop) + 1004  [0x102fcf8d8]  core_frontier.rs:67
                            + !   : | + ! : 1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent5slice  (in deslop) + 1472  [0x102fcfaac]  core_frontier.rs:69
                            + !   : | + ! :   1 _xzm_free  (in libsystem_malloc.dylib) + 352  [0x1854b0278]
                            + !   : | + ! 4 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 496  [0x103024a30]  core_frontier.rs:151
                            + !   : | + ! : 2 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent6append  (in deslop) + 892  [0x102fcffbc]  core_frontier.rs:98
                            + !   : | + ! : | 2 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + ! : |   2 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + ! : |     2 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + ! : |       2 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | + ! : |         1 xzm_realloc  (in libsystem_malloc.dylib) + 904  [0x1854aff0c]
                            + !   : | + ! : |         + 1 _xzm_free  (in libsystem_malloc.dylib) + 960  [0x1854b04d8]
                            + !   : | + ! : |         1 xzm_realloc  (in libsystem_malloc.dylib) + 60  [0x1854afbc0]
                            + !   : | + ! : 1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent6append  (in deslop) + 228  [0x102fcfd24]  core_frontier.rs:98
                            + !   : | + ! : | 1 _platform_memmove  (in libsystem_platform.dylib) + 428  [0x18569d50c]
                            + !   : | + ! : 1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + !   : | + ! :   1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + !   : | + ! 2 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 196  [0x103024904]  core_frontier.rs:149
                            + !   : | + ! : 1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent5slice  (in deslop) + 428  [0x102fcf698]  core_frontier.rs:58
                            + !   : | + ! : | 1 <deduplicated_symbol>  (in libsystem_malloc.dylib) + 112  [0x1854af1cc]
                            + !   : | + ! : 1 _xzm_free  (in libsystem_malloc.dylib) + 1568  [0x1854b0738]
                            + !   : | + ! 2 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 624  [0x103024ab0]  core_frontier.rs:153
                            + !   : | + ! : 2 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | + ! :   1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 76  [0x102e26aac]  lib.rs:546
                            + !   : | + ! :   | 1 _platform_memmove  (in libsystem_platform.dylib) + 428  [0x18569d50c]
                            + !   : | + ! :   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2244  [0x102e27f74]  portable.rs:83
                            + !   : | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 476  [0x103024a1c]  core_frontier.rs:151
                            + !   : | + !   1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent6append  (in deslop) + 864  [0x102fcffa0]  core_frontier.rs:97
                            + !   : | + !     1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + !       1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + !         1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + !           1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | + !             1 xzm_realloc  (in libsystem_malloc.dylib) + 880  [0x1854afef4]
                            + !   : | + !               1 _platform_memmove  (in libsystem_platform.dylib) + 204  [0x18569d42c]
                            + !   : | + 6 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 84  [0x102ef23a4]  cache.rs:139
                            + !   : | + ! 6 _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 76  [0x102f1040c]  content.rs:315
                            + !   : | + !   3 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 172  [0x103023c48]  call_targets.rs:292
                            + !   : | + !   : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 816  [0x102fa9410]  corroboration.rs:26
                            + !   : | + !   : | 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 24  [0x102fdad9c]  map.rs:1935
                            + !   : | + !   : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 956  [0x102fa949c]  corroboration.rs:27
                            + !   : | + !   : 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 340  [0x102fdaed8]  map.rs:1942
                            + !   : | + !   1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 344  [0x102fdaedc]  map.rs:1942
                            + !   : | + !   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 132  [0x103023c20]  call_targets.rs:292
                            + !   : | + !   : 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10population  (in deslop) + 312  [0x103025974]  frontier.rs:139
                            + !   : | + !   :   1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | + !   :     1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + !   :       1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + !   :         1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | + !   :           1 xzm_realloc  (in libsystem_malloc.dylib) + 316  [0x1854afcc0]
                            + !   : | + !   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 304  [0x103023ccc]  call_targets.rs:292
                            + !   : | + 1 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 252  [0x102ef244c]  cache.rs:139
                            + !   : | + ! 1 _xzm_free  (in libsystem_malloc.dylib) + 352  [0x1854b0278]
                            + !   : | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            + !   : | +   1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 116  [0x102f1f170]  literal_echo.rs:41
                            + !   : | +     1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 936  [0x102fa9488]  corroboration.rs:27
                            + !   : | 51 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1372  [0x102fae31c]  pair_compare.rs:153
                            + !   : | + 38 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 80  [0x102fbccac]  core.rs:422
                            + !   : | + ! 38 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer23emitted_single_endpoint  (in deslop) + 208  [0x102fbcf70]  core.rs:436
                            + !   : | + !   33 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : 31 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | 28 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + 25 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + ! 25 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !   25 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !     25 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       20 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : 18 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | 16 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | + 16 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   12 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! 11 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! : 11 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   10 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | 7 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | + 7 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +   7 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +     7 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +       7 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +         7 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           6 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           ! 6 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   : 5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :   5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :     5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :       5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :         5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :           5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :             5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               4 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               | 4 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   + 3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +     3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +       3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         ! 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !     2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !         2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !           2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !             2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !               2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                     2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                         2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                           2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                             2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                               2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                     2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                         2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                           : 1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 1180  [0x102fdc4fc]  fingerprint.rs:282
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                           :   1 _RNvMs4_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB5_6RawVecNtNtCs7J0RnZC7BVr_11deslop_core11boilerplate16BoilerplateRangeE8grow_oneBQ_  (in deslop) + 60  [0x10327eff0]  mod.rs:188
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                           :     1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                           :       1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                           :         1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 156  [0x1854bf700]
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         !                                                                                       1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 3236  [0x102e28354]  portable.rs:83
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +           1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   +             1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2576  [0x102e280c0]  portable.rs:83
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |     1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |       1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |         1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |           1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 320  [0x102e26ba0]  lib.rs:577
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               |             1 _platform_memmove  (in libsystem_platform.dylib) + 460  [0x18569d52c]
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !   : | + !       : | +   ! :   | +           !   :                 1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2456  [0x102e28048]  portable.rs:83
                            + !   : | + !   : | + !       : | +   ! :   | +           !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !   : | + !       : | +   ! :   | +           !     1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2636  [0x102e280fc]  portable.rs:83
                            + !   : | + !   : | + !       : | +   ! :   | +           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !   : | + !       : | +   ! :   | +             1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1580  [0x102e27cdc]  portable.rs:83
                            + !   : | + !   : | + !       : | +   ! :   | 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !   : | + !       : | +   ! :   | + 2 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2336,2392  [0x102e27fd0,0x102e28008]  portable.rs:83
                            + !   : | + !   : | + !       : | +   ! :   | 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 40  [0x102fee8a4]  tokens.rs:157
                            + !   : | + !   : | + !       : | +   ! :   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + !       : | +   ! :     1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : | + !       : | +   ! :       1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 676  [0x102fdc304]  fingerprint.rs:282
                            + !   : | + !   : | + !       : | +   ! :         1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 852  [0x102e272d8]  lib.rs:1205
                            + !   : | + !   : | + !       : | +   ! :           1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 320  [0x102e26ba0]  lib.rs:577
                            + !   : | + !   : | + !       : | +   ! :             1 DYLD-STUB$$memcpy  (in deslop) + 4  [0x10329cda8]
                            + !   : | + !   : | + !       : | +   ! 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !   : | + !       : | +   !   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 260  [0x102e277b4]  portable.rs:83
                            + !   : | + !   : | + !       : | +   4 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + !       : | +     4 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : | + !       : | +       2 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 952,2700  [0x102e27a68,0x102e2813c]  portable.rs:83
                            + !   : | + !   : | + !       : | +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 676  [0x102fdc304]  fingerprint.rs:282
                            + !   : | + !   : | + !       : | +       : 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 852  [0x102e272d8]  lib.rs:1205
                            + !   : | + !   : | + !       : | +       :   1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 320  [0x102e26ba0]  lib.rs:577
                            + !   : | + !   : | + !       : | +       :     1 _platform_memmove  (in libsystem_platform.dylib) + 0  [0x18569d360]
                            + !   : | + !   : | + !       : | +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 696  [0x102fdc318]  fingerprint.rs:282
                            + !   : | + !   : | + !       : | +         1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | + !   : | + !       : | +           1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 28  [0x102e26a7c]  lib.rs:545
                            + !   : | + !   : | + !       : | 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + !       : |   1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : | + !       : |   ! 1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2464  [0x102e28050]  portable.rs:83
                            + !   : | + !   : | + !       : |   1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 212  [0x102ff0420]  sibling.rs:271
                            + !   : | + !   : | + !       : |     1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2852  [0x102e281d4]  portable.rs:83
                            + !   : | + !   : | + !       : 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + !       :   1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 212  [0x102ff0420]  sibling.rs:271
                            + !   : | + !   : | + !       :   + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling11hash_window  (in deslop) + 156  [0x102ff0254]  sibling.rs:248
                            + !   : | + !   : | + !       :   +   1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 48  [0x102e26fb4]  lib.rs:1205
                            + !   : | + !   : | + !       :   1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 240  [0x102ff043c]  sibling.rs:272
                            + !   : | + !   : | + !       :     1 _xzm_free  (in libsystem_malloc.dylib) + 352  [0x1854b0278]
                            + !   : | + !   : | + !       :       1 _platform_memset  (in libsystem_platform.dylib) + 140  [0x18569d11c]
                            + !   : | + !   : | + !       5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + !         4 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : | + !         | 2 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1252,1816  [0x102e27b94,0x102e27dc8]  portable.rs:83
                            + !   : | + !   : | + !         | 1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            + !   : | + !   : | + !         | + 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 48  [0x102e26fb4]  lib.rs:1205
                            + !   : | + !   : | + !         | 1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 548  [0x102fdc284]  fingerprint.rs:282
                            + !   : | + !   : | + !         1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 212  [0x102ff0420]  sibling.rs:271
                            + !   : | + !   : | + !           1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1676  [0x102e27d3c]  portable.rs:83
                            + !   : | + !   : | + 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : | + ! 2 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : | + !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            + !   : | + !   : | + !   : 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | + !   : | + !   :   1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 76  [0x102e26aac]  lib.rs:546
                            + !   : | + !   : | + !   :     1 DYLD-STUB$$memcpy  (in deslop) + 4  [0x10329cda8]
                            + !   : | + !   : | + !   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2692  [0x102e28134]  portable.rs:83
                            + !   : | + !   : | + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !   : | +   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 3208  [0x102e28338]  portable.rs:83
                            + !   : | + !   : | 3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : |   3 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   : |     1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 676  [0x102fdc304]  fingerprint.rs:282
                            + !   : | + !   : |     ! 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 572  [0x102e271c0]  lib.rs:1205
                            + !   : | + !   : |     1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            + !   : | + !   : |     ! 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | + !   : |     !   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1284  [0x102e27bb4]  portable.rs:83
                            + !   : | + !   : |     1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1252  [0x102e27b94]  portable.rs:83
                            + !   : | + !   : 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 84  [0x102fee8d0]  tokens.rs:161
                            + !   : | + !   : | 1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |   1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |     1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |       1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |         1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |           1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |             1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |               1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |                 1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
                            + !   : | + !   : |                   1 _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 16  [0x102fc7470]  ast.rs:133
                            + !   : | + !   : 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !   :   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1252  [0x102e27b94]  portable.rs:83
                            + !   : | + !   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !   : 3 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !   :   2 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1072,3392  [0x102e27ae0,0x102e283f0]  portable.rs:83
                            + !   : | + !   :   1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 884  [0x102e272f8]  lib.rs:1206
                            + !   : | + !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 276  [0x102fee990]  tokens.rs:164
                            + !   : | + !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 680  [0x102feeb24]  tokens.rs:169
                            + !   : | + 6 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 100  [0x102fbccc0]  core.rs:423
                            + !   : | + ! 6 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer23emitted_single_endpoint  (in deslop) + 208  [0x102fbcf70]  core.rs:436
                            + !   : | + !   6 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !     6 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       4 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       : 4 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :   4 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     | 3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |     3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |       3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |         3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |           3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |             3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |               3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 + 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !       :     |                 +     ! 1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            + !   : | + !       :     |                 +     !   1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 20  [0x102e26f98]  lib.rs:1204
                            + !   : | + !       :     |                 +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       :     |                 +                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !       :     |                 +                                                                       1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 884  [0x102e272f8]  lib.rs:1206
                            + !   : | + !       :     |                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !       :     |                   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 672  [0x102e27950]  portable.rs:83
                            + !   : | + !       :     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !       :       1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 676  [0x102fdc304]  fingerprint.rs:282
                            + !   : | + !       :         1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 48  [0x102e26fb4]  lib.rs:1205
                            + !   : | + !       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + !         1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            + !   : | + !         | 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | + !         |   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1844  [0x102e27de4]  portable.rs:83
                            + !   : | + !         1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 44  [0x102e276dc]  portable.rs:83
                            + !   : | + 3 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 176  [0x102fbcd0c]  core.rs:427
                            + !   : | + ! 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer8resolved  (in deslop) + 108  [0x102fbd0a0]  core.rs:450
                            + !   : | + ! : 1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRTNtNtCs7J0RnZC7BVr_11deslop_core5state6FileIdjjAhj20_jEEB1K_  (in deslop) + 140  [0x102f86d28]  mod.rs:700
                            + !   : | + ! :   1 _RNvXs3_NtNtCs3bJEMBXHbwl_4core4hash3sipINtB5_6HasherNtB5_11Sip13RoundsENtB7_6Hasher5writeCs7J0RnZC7BVr_11deslop_core  (in deslop) + 288  [0x1030604c4]  sip.rs:271
                            + !   : | + ! 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer8resolved  (in deslop) + 532  [0x102fbd248]  core.rs:453
                            + !   : | + ! : 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! :   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! :     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + ! :       1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + ! :         1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2548  [0x102e280a4]  portable.rs:83
                            + !   : | + ! 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer8resolved  (in deslop) + 708  [0x102fbd2f8]  core.rs:453
                            + !   : | + !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint23is_literal_data_subtree  (in deslop) + 212  [0x102fdcb70]  fingerprint.rs:381
                            + !   : | + 2 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 196  [0x102fbcd20]  core.rs:427
                            + !   : | + ! 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer8resolved  (in deslop) + 108  [0x102fbd0a0]  core.rs:450
                            + !   : | + ! : 1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRTNtNtCs7J0RnZC7BVr_11deslop_core5state6FileIdjjAhj20_jEEB1K_  (in deslop) + 92  [0x102f86cf8]  mod.rs:700
                            + !   : | + ! :   1 _RNvXs3_NtNtCs3bJEMBXHbwl_4core4hash3sipINtB5_6HasherNtB5_11Sip13RoundsENtB7_6Hasher5writeCs7J0RnZC7BVr_11deslop_core  (in deslop) + 12  [0x1030603b0]  sip.rs:262
                            + !   : | + ! 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer8resolved  (in deslop) + 1188  [0x102fbd4d8]  core.rs:453
                            + !   : | + !   1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | + !     1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 320  [0x102e26ba0]  lib.rs:577
                            + !   : | + !       1 DYLD-STUB$$memcpy  (in deslop) + 4  [0x10329cda8]
                            + !   : | + 2 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 252  [0x102fbcd58]  core.rs:428
                            + !   : | +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7overlap4core14align_siblings  (in deslop) + 5308  [0x103029b04]  core.rs:155
                            + !   : | +   : 1 _RNvMs4_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB5_6RawVecINtNtCs3bJEMBXHbwl_4core6option6OptionINtNtB7_4sync3ArceEEE8grow_oneCsavjJHO1fFjO_14regex_automata  (in deslop) + 60  [0x10328f820]  mod.rs:188
                            + !   : | +   :   1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCsavjJHO1fFjO_14regex_automata  (in deslop) + 140  [0x10328fc6c]  mod.rs:563
                            + !   : | +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7overlap4core14align_siblings  (in deslop) + 1392  [0x103028bb8]  core.rs:163
                            + !   : | 9 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 580  [0x102fae004]  pair_compare.rs:108
                            + !   : | + 9 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer7overlap  (in deslop) + 44  [0x102f96ea4]  overlap.rs:209
                            + !   : | +   7 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer9view_pair  (in deslop) + 52  [0x102f97164]  overlap.rs:410
                            + !   : | +   ! 4 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 516  [0x102f96198]  overlap.rs:441
                            + !   : | +   ! : 4 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   | 3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |     3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |       3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |         3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |           3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |             3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |               3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                 3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                     3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                       3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                         3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                           3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | +   ! :   |                             + ! 1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 696  [0x102fdc318]  fingerprint.rs:282
                            + !   : | +   ! :   |                             + !   1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | +   ! :   |                             + !     1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 76  [0x102e26aac]  lib.rs:546
                            + !   : | +   ! :   |                             + !       1 _platform_memmove  (in libsystem_platform.dylib) + 12  [0x18569d36c]
                            + !   : | +   ! :   |                             + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             +         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             +           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             +             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | +   ! :   |                             +               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | +   ! :   |                             +                 1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 44  [0x102e276dc]  portable.rs:83
                            + !   : | +   ! :   |                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | +   ! :   |                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 696  [0x102fdc318]  fingerprint.rs:282
                            + !   : | +   ! :   |                                 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | +   ! :   |                                   1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 320  [0x102e26ba0]  lib.rs:577
                            + !   : | +   ! :   |                                     1 _platform_memmove  (in libsystem_platform.dylib) + 0  [0x18569d360]
                            + !   : | +   ! :   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | +   ! :     1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 732  [0x102e2798c]  portable.rs:83
                            + !   : | +   ! 1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 2128  [0x102f967e4]  overlap.rs:441
                            + !   : | +   ! : 1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint20collect_fingerprints  (in deslop) + 588  [0x102fdc7d4]  fingerprint.rs:37
                            + !   : | +   ! :   1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | +   ! :     1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 76  [0x102e26aac]  lib.rs:546
                            + !   : | +   ! :       1 _platform_memmove  (in libsystem_platform.dylib) + 428  [0x18569d50c]
                            + !   : | +   ! 1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 2220  [0x102f96840]  overlap.rs:441
                            + !   : | +   ! : 1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + !   : | +   ! :   1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | +   ! :     1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | +   ! :       1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | +   ! :         1 xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
                            + !   : | +   ! :           1 _xzm_xzone_malloc_freelist_outlined  (in libsystem_malloc.dylib) + 324  [0x1854b67e4]
                            + !   : | +   ! :             1 _xzm_xzone_find_and_malloc_from_freelist_chunk  (in libsystem_malloc.dylib) + 860  [0x1854b72d8]
                            + !   : | +   ! :               1 xzm_chunk_mark_used  (in libsystem_malloc.dylib) + 188  [0x185497efc]
                            + !   : | +   ! :                 1 _xzm_reclaim_mark_used  (in libsystem_malloc.dylib) + 176  [0x185497fe0]
                            + !   : | +   ! :                   1 mach_vm_reclaim_update_kernel_accounting  (in libsystem_kernel.dylib) + 92  [0x185663494]
                            + !   : | +   ! :                     1 mach_vm_reclaim_update_kernel_accounting_trap  (in libsystem_kernel.dylib) + 8  [0x18564fcb8]
                            + !   : | +   ! 1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 1436  [0x102f96530]  overlap.rs:441
                            + !   : | +   2 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer9view_pair  (in deslop) + 32  [0x102f97150]  overlap.rs:410
                            + !   : | +     1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 96  [0x102f95ff4]  overlap.rs:438
                            + !   : | +     : 1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRTNtNtCs7J0RnZC7BVr_11deslop_core5state6FileIdjjAhj20_jEEB1K_  (in deslop) + 204  [0x102f86d68]  mod.rs:700
                            + !   : | +     :   1 _RNvXs3_NtNtCs3bJEMBXHbwl_4core4hash3sipINtB5_6HasherNtB5_11Sip13RoundsENtB7_6Hasher5writeCs7J0RnZC7BVr_11deslop_core  (in deslop) + 288  [0x1030604c4]  sip.rs:271
                            + !   : | +     1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 2128  [0x102f967e4]  overlap.rs:441
                            + !   : | +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint20collect_fingerprints  (in deslop) + 196  [0x102fdc64c]  fingerprint.rs:37
                            + !   : | +         1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x10327f9d4]
                            + !   : | 8 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 664  [0x102fae058]  pair_compare.rs:110
                            + !   : | + 6 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 844  [0x102eecddc]  cache.rs:40
                            + !   : | + ! 4 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens16collapsed_leaves  (in deslop) + 48  [0x102fee540]  tokens.rs:198
                            + !   : | + ! : 3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : | 3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + ! : |   + ! 1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + ! : |   + !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 776  [0x102fdc368]  fingerprint.rs:282
                            + !   : | + ! : |   + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + ! : |   +                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + !   : | + ! : |   +                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            + !   : | + ! : |   +                                                                 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + !   : | + ! : |   +                                                                   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 3124  [0x102e282e4]  portable.rs:83
                            + !   : | + ! : |   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + ! : |     1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + ! : |       1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 3152  [0x102e28300]  portable.rs:83
                            + !   : | + ! : 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + ! :   1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + ! :     1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 884  [0x102e272f8]  lib.rs:1206
                            + !   : | + ! 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens16collapsed_leaves  (in deslop) + 116  [0x102fee584]  tokens.rs:202
                            + !   : | + !   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            + !   : | + !     2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            + !   : | + !       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            + !   : | + !         2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            + !   : | + !           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            + !   : | + !           | 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 88  [0x102fee664]  tokens.rs:235
                            + !   : | + !           |   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11boilerplate34is_import_boilerplate_only_subtree  (in deslop) + 568  [0x102fdbe50]  boilerplate.rs:56
                            + !   : | + !           |     1 _RNvNtCs7J0RnZC7BVr_11deslop_core11boilerplate34is_import_boilerplate_only_subtree  (in deslop) + 568  [0x102fdbe50]  boilerplate.rs:56
                            + !   : | + !           |       1 _RNvNtCs7J0RnZC7BVr_11deslop_core11boilerplate34is_import_boilerplate_only_subtree  (in deslop) + 528  [0x102fdbe28]  boilerplate.rs:52
                            + !   : | + !           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 212  [0x102fee6e0]  tokens.rs:248
                            + !   : | + 1 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 1048  [0x102eecea8]  cache.rs:40
                            + !   : | + ! 1 _RNvXNtNtCs3bJEMBXHbwl_4core4iter8adaptersINtB2_12GenericShuntINtNtB2_3map3MapINtNtNtB6_5slice4iter4IterNtNtCs7J0RnZC7BVr_11deslop_core6tokens13CollapsedLeafENCINvNtNtB1H_7content8frontier14member_contentNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB3f_Es_0EINtNtB6_6option6OptionNtNtB6_7convert10InfallibleEENtNtNtB4_6traits8iterator8Iterator4nextB1H_  (in deslop) + 248  [0x1030458d0]  mod.rs:177
                            + !   : | + !   1 _RNvCs98dNin8E3s3_6blake34hash  (in deslop) + 192  [0x102e25b8c]  lib.rs:921
                            + !   : | + !     1 _RINvCs98dNin8E3s3_6blake331compress_subtree_to_parent_nodeNtNtB2_4join10SerialJoinEB2_  (in deslop) + 92  [0x102e25740]
                            + !   : | + !       1 _RINvCs98dNin8E3s3_6blake321compress_subtree_wideNtNtB2_4join10SerialJoinEB2_  (in deslop) + 224  [0x102e25240]
                            + !   : | + !         1 _RINvCs98dNin8E3s3_6blake321compress_subtree_wideNtNtB2_4join10SerialJoinEB2_  (in deslop) + 224  [0x102e25240]
                            + !   : | + !           1 _RINvCs98dNin8E3s3_6blake321compress_subtree_wideNtNtB2_4join10SerialJoinEB2_  (in deslop) + 268  [0x102e2526c]
                            + !   : | + !             1 _RINvCs98dNin8E3s3_6blake321compress_subtree_wideNtNtB2_4join10SerialJoinEB2_  (in deslop) + 224  [0x102e25240]
                            + !   : | + !               1 _RINvCs98dNin8E3s3_6blake321compress_subtree_wideNtNtB2_4join10SerialJoinEB2_  (in deslop) + 268  [0x102e2526c]
                            + !   : | + !                 1 _RINvCs98dNin8E3s3_6blake321compress_subtree_wideNtNtB2_4join10SerialJoinEB2_  (in deslop) + 224  [0x102e25240]
                            + !   : | + !                   1 _RINvCs98dNin8E3s3_6blake321compress_subtree_wideNtNtB2_4join10SerialJoinEB2_  (in deslop) + 552  [0x102e25388]
                            + !   : | + !                     1 _RINvCs98dNin8E3s3_6blake321compress_subtree_wideNtNtB2_4join10SerialJoinEB2_  (in deslop) + 552  [0x102e25388]
                            + !   : | + !                       1 blake3_hash_many_neon  (in deslop) + 4096  [0x1032590a8]
                            + !   : | + 1 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 2136  [0x102eed2e8]  cache.rs:40
                            + !   : | +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets16external_targets  (in deslop) + 256  [0x1030227c4]  call_targets.rs:155
                            + !   : | +     1 _RNvMs4_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB5_6RawVecRNtCslTxlaW6rlF6_11tree_sitter4NodeE8grow_oneCs7J0RnZC7BVr_11deslop_core  (in deslop) + 60  [0x10327f438]  mod.rs:188
                            + !   : | +       1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + !   : | +         1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + !   : | +           1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + !   : | +             1 xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
                            + !   : | +               1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 88  [0x1854b4f94]
                            + !   : | 3 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 188  [0x102fade7c]  pair_compare.rs:105
                            + !   : | + 3 _RNvNtNtCs7J0RnZC7BVr_11deslop_core8pipeline15embedding_batch11snippet_for  (in deslop) + 276  [0x10302f034]  embedding_batch.rs:276
                            + !   : | +   3 _RNvMNtCs8Hh7wWMBHrQ_5alloc6stringNtB2_6String15from_utf8_lossy  (in deslop) + 52  [0x102e219c8]  string.rs:631
                            + !   : | +     2 _RNvXs2_NtNtCs3bJEMBXHbwl_4core3str5lossyNtB5_10Utf8ChunksNtNtNtNtB9_4iter6traits8iterator8Iterator4next  (in deslop) + 40  [0x102e65ebc]  lossy.rs:0
                            + !   : | +     1 _RNvXs2_NtNtCs3bJEMBXHbwl_4core3str5lossyNtB5_10Utf8ChunksNtNtNtNtB9_4iter6traits8iterator8Iterator4next  (in deslop) + 408  [0x102e6602c]  lossy.rs:277
                            + !   : | 3 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 632  [0x102fae038]  pair_compare.rs:110
                            + !   : | + 2 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 844  [0x102eecddc]  cache.rs:40
                            + !   : | + ! 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens16collapsed_leaves  (in deslop) + 48  [0x102fee540]  tokens.rs:198
                            + !   : | + !   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !     2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !         : 1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !         :   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 808  [0x102fdc388]  fingerprint.rs:282
                            + !   : | + !         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + !   : | + !                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + !   : | + !                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + !   : | + !                           1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2032  [0x102e27ea0]  portable.rs:83
                            + !   : | + 1 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 200  [0x102eecb58]  cache.rs:33
                            + !   : | 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1984  [0x102fae580]  pair_compare.rs:105
                            + !   : | + 2 _RNvNtCs7J0RnZC7BVr_11deslop_core13report_render23canonicalise_whitespace  (in deslop) + 148  [0x102fdd4ec]  report_render.rs:245
                            + !   : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 152  [0x102fade58]  pair_compare.rs:105
                            + !   : | + 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core8pipeline15embedding_batch11snippet_for  (in deslop) + 276  [0x10302f034]  embedding_batch.rs:276
                            + !   : | +   1 _RNvMNtCs8Hh7wWMBHrQ_5alloc6stringNtB2_6String15from_utf8_lossy  (in deslop) + 52  [0x102e219c8]  string.rs:631
                            + !   : | +     1 _RNvXs2_NtNtCs3bJEMBXHbwl_4core3str5lossyNtB5_10Utf8ChunksNtNtNtNtB9_4iter6traits8iterator8Iterator4next  (in deslop) + 40  [0x102e65ebc]  lossy.rs:0
                            + !   : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1968  [0x102fae570]  pair_compare.rs:105
                            + !   : | + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core13report_render23canonicalise_whitespace  (in deslop) + 184  [0x102fdd510]  report_render.rs:0
                            + !   : | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1656  [0x102fae438]  pair_compare.rs:153
                            + !   : | + 1 _xzm_free  (in libsystem_malloc.dylib) + 20  [0x1854b012c]
                            + !   : | 1 _xzm_free  (in libsystem_malloc.dylib) + 1568  [0x1854b0738]
                            + !   : 2 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 152  [0x102fd4eec]  cluster_kind.rs:221
                            + !   : | 1 _RNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare13text_identity30same_source_bytes_and_language  (in deslop) + 296  [0x103042d9c]  text_identity.rs:17
                            + !   : | 1 _RNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare13text_identity30same_source_bytes_and_language  (in deslop) + 492  [0x103042e60]  text_identity.rs:25
                            + !   : |   1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRNtNtCs7J0RnZC7BVr_11deslop_core5state6FileIdEB1J_  (in deslop) + 168  [0x102f86588]  mod.rs:701
                            + !   : 1 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 860  [0x102fd51b0]  cluster_kind.rs:225
                            + !   : | 1 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 916  [0x102fd51e8]  cluster_kind.rs:225
                            + !   : 1 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 1520  [0x102fd5444]  cluster_kind.rs:226
                            + !   :   1 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare9admissionNtB4_14AdmissionFacts4from  (in deslop) + 1164  [0x102fd62cc]  admission.rs:62
                            + !   1 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer9pair_kind  (in deslop) + 1320  [0x102fd5cbc]  cluster_kind.rs:207
                            + 67 _RNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information22family_recovery_groups  (in deslop) + 820  [0x103044408]  information.rs:215
                            + ! 39 _RNvXs1_NtNtNtCs3bJEMBXHbwl_4core4iter8adapters7flattenINtB5_7FlatMapINtNtNtBb_5slice4iter4IterINtNtCs8Hh7wWMBHrQ_5alloc3vec3VecjEEIB1v_B1u_ENCNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information22family_recovery_groupss_0ENtNtNtB9_6traits8iterator8Iterator4nextB2q_  (in deslop) + 900  [0x10305728c]  flatten.rs:64
                            + ! : 39 _RNvXs0_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB5_19ClusterKindMeasurerNtNtBd_7cluster16ClusterKindJudge4kind  (in deslop) + 288  [0x103055160]  cluster_kind.rs:352
                            + ! :   39 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer9pair_kind  (in deslop) + 456  [0x102fd595c]  cluster_kind.rs:203
                            + ! :     39 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 1204  [0x102fd5308]  cluster_kind.rs:225
                            + ! :       13 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 740  [0x102fae0a4]  pair_compare.rs:116
                            + ! :       | 3 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            + ! :       | + 2 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + ! :       | + ! 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + ! :       | + ! : 1 _platform_memcmp  (in libsystem_platform.dylib) + 236  [0x18569affc]
                            + ! :       | + ! 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 924  [0x10303ff9c]  literal_echo.rs:164
                            + ! :       | + !   1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + ! :       | + !     1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + ! :       | + !       1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + ! :       | + !         1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + ! :       | + !           1 xzm_realloc  (in libsystem_malloc.dylib) + 1084  [0x1854affc0]
                            + ! :       | + 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 100  [0x102f1f160]  literal_echo.rs:41
                            + ! :       | +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17substituted_pairs  (in deslop) + 188  [0x1030257d8]  rename.rs:374
                            + ! :       | +     1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + ! :       | +       1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + ! :       | +         1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + ! :       | +           1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + ! :       | +             1 xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
                            + ! :       | +               1 <deduplicated_symbol>  (in libsystem_malloc.dylib) + 212  [0x1854af230]
                            + ! :       | 3 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 336  [0x102f0c510]  content.rs:257
                            + ! :       | + 2 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 708  [0x102fa1778]  rename.rs:182
                            + ! :       | + ! 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 856  [0x102fa9438]  corroboration.rs:26
                            + ! :       | + ! : 1 _RINvMsi_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB6_8BTreeMapyyE27bulk_build_from_sorted_iterINtNtBc_3vec3VecTyyEEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 232  [0x102ef5a88]  map.rs:1716
                            + ! :       | + ! 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 340  [0x102fdaed8]  map.rs:1942
                            + ! :       | + 1 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 692  [0x102fa1768]  rename.rs:182
                            + ! :       | +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17substituted_pairs  (in deslop) + 188  [0x1030257d8]  rename.rs:374
                            + ! :       | +     1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + ! :       | +       1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + ! :       | +         1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + ! :       | +           1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + ! :       | +             1 xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
                            + ! :       | +               1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 516  [0x1854b5140]
                            + ! :       | 3 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
                            + ! :       | + 2 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + ! :       | + ! 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + ! :       | + ! : 1 _platform_memcmp  (in libsystem_platform.dylib) + 236  [0x18569affc]
                            + ! :       | + ! 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + ! :       | + 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 848  [0x102f1f44c]  literal_echo.rs:42
                            + ! :       | +   1 _RINvNtNtCs7J0RnZC7BVr_11deslop_core7content8frontier10leaf_bytesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB6_  (in deslop) + 68  [0x102f16d78]  frontier.rs:123
                            + ! :       | +     1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRNtNtCs7J0RnZC7BVr_11deslop_core5state6FileIdEB1J_  (in deslop) + 264  [0x102f865e8]  mod.rs:702
                            + ! :       | 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 80  [0x102f0c410]  content.rs:254
                            + ! :       | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 76  [0x102f1040c]  content.rs:315
                            + ! :       | +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 2164  [0x103024410]  call_targets.rs:292
                            + ! :       | +     1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + ! :       | +       1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + ! :       | +         1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + ! :       | 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 2392  [0x102f0cd18]  content.rs:264
                            + ! :       | + 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename10consistent20strip_common_affixes  (in deslop) + 316  [0x10303fb98]  consistent.rs:169
                            + ! :       | +   1 _platform_memmove  (in libsystem_platform.dylib) + 428  [0x18569d50c]
                            + ! :       | 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9668  [0x102f0e984]  content.rs:264
                            + ! :       | + 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + ! :       | +   1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 856  [0x102fa9438]  corroboration.rs:26
                            + ! :       | +     1 _RINvMsi_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB6_8BTreeMapyyE27bulk_build_from_sorted_iterINtNtBc_3vec3VecTyyEEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 48  [0x102ef59d0]  map.rs:1714
                            + ! :       | +       1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 420  [0x1854b50e0]
                            + ! :       | 1 _xzm_free  (in libsystem_malloc.dylib) + 1568  [0x1854b0738]
                            + ! :       13 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1812  [0x102fae4d4]  pair_compare.rs:153
                            + ! :       | 9 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 228  [0x102ef2434]  cache.rs:139
                            + ! :       | + 4 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            + ! :       | + ! 4 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + ! :       | + !   2 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + ! :       | + !   : 2 _platform_memcmp  (in libsystem_platform.dylib) + 236  [0x18569affc]
                            + ! :       | + !   1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + ! :       | + !   1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 924  [0x10303ff9c]  literal_echo.rs:164
                            + ! :       | + !     1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + ! :       | + !       1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + ! :       | + !         1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + ! :       | + !           1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + ! :       | + !             1 xzm_realloc  (in libsystem_malloc.dylib) + 904  [0x1854aff0c]
                            + ! :       | + !               1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + ! :       | + !                 1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + ! :       | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 80  [0x102f0c410]  content.rs:254
                            + ! :       | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 76  [0x102f1040c]  content.rs:315
                            + ! :       | + !   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 1540  [0x1030241a0]  call_targets.rs:292
                            + ! :       | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 296  [0x102f0c4e8]  content.rs:257
                            + ! :       | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + ! :       | + !   1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1072  [0x102fa9510]  corroboration.rs:27
                            + ! :       | + !     1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 32  [0x102fc1020]  entry.rs:145
                            + ! :       | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 336  [0x102f0c510]  content.rs:257
                            + ! :       | + ! 1 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 708  [0x102fa1778]  rename.rs:182
                            + ! :       | + !   1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryyTyjEE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 1276  [0x102fc1bfc]  entry.rs:149
                            + ! :       | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
                            + ! :       | + ! 1 _xzm_free  (in libsystem_malloc.dylib) + 1568  [0x1854b0738]
                            + ! :       | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9668  [0x102f0e984]  content.rs:264
                            + ! :       | +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + ! :       | +     1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 344  [0x102fdaedc]  map.rs:1942
                            + ! :       | 3 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 196  [0x102ef2414]  cache.rs:139
                            + ! :       | + 2 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 196  [0x103024904]  core_frontier.rs:149
                            + ! :       | + ! 1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent5slice  (in deslop) + 36  [0x102fcf510]  core_frontier.rs:48
                            + ! :       | + ! 1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent5slice  (in deslop) + 428  [0x102fcf698]  core_frontier.rs:58
                            + ! :       | + !   1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 268  [0x1854b5048]
                            + ! :       | + 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 476  [0x103024a1c]  core_frontier.rs:151
                            + ! :       | +   1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent6append  (in deslop) + 228  [0x102fcfd24]  core_frontier.rs:98
                            + ! :       | +     1 _platform_memmove  (in libsystem_platform.dylib) + 204  [0x18569d42c]
                            + ! :       | 1 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 84  [0x102ef23a4]  cache.rs:139
                            + ! :       |   1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 76  [0x102f1040c]  content.rs:315
                            + ! :       |     1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 172  [0x103023c48]  call_targets.rs:292
                            + ! :       |       1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1072  [0x102fa9510]  corroboration.rs:27
                            + ! :       |         1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 464  [0x102fc11d0]  entry.rs:147
                            + ! :       |           1 _platform_memmove  (in libsystem_platform.dylib) + 436  [0x18569d514]
                            + ! :       6 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1372  [0x102fae31c]  pair_compare.rs:153
                            + ! :       | 6 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 80  [0x102fbccac]  core.rs:422
                            + ! :       |   6 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer23emitted_single_endpoint  (in deslop) + 208  [0x102fbcf70]  core.rs:436
                            + ! :       |     5 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + ! :       |     + 5 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + ! :       |     +   3 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 116,672,...  [0x102e27724,0x102e27950,...]  portable.rs:83
                            + ! :       |     +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            + ! :       |     +   ! 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + ! :       |     +   !   1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 76  [0x102e26aac]  lib.rs:546
                            + ! :       |     +   !     1 _platform_memmove  (in libsystem_platform.dylib) + 428  [0x18569d50c]
                            + ! :       |     +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 808  [0x102fdc388]  fingerprint.rs:282
                            + ! :       |     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |                                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + ! :       |                                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 212  [0x102ff0420]  sibling.rs:271
                            + ! :       |                                                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling11hash_window  (in deslop) + 240  [0x102ff02a8]  sibling.rs:253
                            + ! :       |                                                                                         1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            + ! :       |                                                                                           1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 76  [0x102e26aac]  lib.rs:546
                            + ! :       |                                                                                             1 _platform_memmove  (in libsystem_platform.dylib) + 460  [0x18569d52c]
                            + ! :       3 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 580  [0x102fae004]  pair_compare.rs:108
                            + ! :       | 3 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer7overlap  (in deslop) + 44  [0x102f96ea4]  overlap.rs:209
                            + ! :       |   3 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer9view_pair  (in deslop) + 52  [0x102f97164]  overlap.rs:410
                            + ! :       |     2 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 516  [0x102f96198]  overlap.rs:441
                            + ! :       |     + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + ! :       |     + ! 1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + ! :       |     + !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 632  [0x102fdc2d8]  fingerprint.rs:282
                            + ! :       |     + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |     +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |     +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |     +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |     +         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :       |     +           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            + ! :       |     +             1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 776  [0x102fdc368]  fingerprint.rs:282
                            + ! :       |     1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 2128  [0x102f967e4]  overlap.rs:441
                            + ! :       |       1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 3568  [0x102e284a0]  portable.rs:93
                            + ! :       3 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 664  [0x102fae058]  pair_compare.rs:110
                            + ! :       | 1 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 16  [0x102eecaa0]  cache.rs:26
                            + ! :       | 1 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 844  [0x102eecddc]  cache.rs:40
                            + ! :       | + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens16collapsed_leaves  (in deslop) + 48  [0x102fee540]  tokens.rs:198
                            + ! :       | +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + ! :       | +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + ! :       | +       1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 520  [0x102e278b8]  portable.rs:83
                            + ! :       | 1 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 1048  [0x102eecea8]  cache.rs:40
                            + ! :       |   1 _RNvXNtNtCs3bJEMBXHbwl_4core4iter8adaptersINtB2_12GenericShuntINtNtB2_3map3MapINtNtNtB6_5slice4iter4IterNtNtCs7J0RnZC7BVr_11deslop_core6tokens13CollapsedLeafENCINvNtNtB1H_7content8frontier14member_contentNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB3f_Es_0EINtNtB6_6option6OptionNtNtB6_7convert10InfallibleEENtNtNtB4_6traits8iterator8Iterator4nextB1H_  (in deslop) + 248  [0x1030458d0]  mod.rs:177
                            + ! :       |     1 _RNvCs98dNin8E3s3_6blake34hash  (in deslop) + 12  [0x102e25ad8]  lib.rs:920
                            + ! :       1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1984  [0x102fae580]  pair_compare.rs:105
                            + ! :         1 _RNvNtCs7J0RnZC7BVr_11deslop_core13report_render23canonicalise_whitespace  (in deslop) + 148  [0x102fdd4ec]  report_render.rs:245
                            + ! 26 _RNvXs1_NtNtNtCs3bJEMBXHbwl_4core4iter8adapters7flattenINtB5_7FlatMapINtNtNtBb_5slice4iter4IterINtNtCs8Hh7wWMBHrQ_5alloc3vec3VecjEEIB1v_B1u_ENCNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information22family_recovery_groupss_0ENtNtNtB9_6traits8iterator8Iterator4nextB2q_  (in deslop) + 528  [0x103057118]  flatten.rs:64
                            + ! : 26 _RNvXs0_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB5_19ClusterKindMeasurerNtNtBd_7cluster16ClusterKindJudge4kind  (in deslop) + 288  [0x103055160]  cluster_kind.rs:352
                            + ! :   26 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer9pair_kind  (in deslop) + 456  [0x102fd595c]  cluster_kind.rs:203
                            + ! :     25 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 1204  [0x102fd5308]  cluster_kind.rs:225
                            + ! :     | 9 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 740  [0x102fae0a4]  pair_compare.rs:116
                            + ! :     | + 4 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
                            + ! :     | + ! 3 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + ! :     | + ! : 2 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 156,172  [0x10303fc9c,0x10303fcac]  literal_echo.rs:154
                            + ! :     | + ! : 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 924  [0x10303ff9c]  literal_echo.rs:164
                            + ! :     | + ! :   1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            + ! :     | + ! :     1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            + ! :     | + ! :       1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            + ! :     | + ! :         1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            + ! :     | + ! :           1 xzm_realloc  (in libsystem_malloc.dylib) + 904  [0x1854aff0c]
                            + ! :     | + ! :             1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + ! :     | + ! :               1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + ! :     | + ! 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 116  [0x102f1f170]  literal_echo.rs:41
                            + ! :     | + !   1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 344  [0x102fdaedc]  map.rs:1942
                            + ! :     | + 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9668  [0x102f0e984]  content.rs:264
                            + ! :     | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + ! :     | + ! : 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1592  [0x102fa9718]  corroboration.rs:27
                            + ! :     | + ! :   1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + ! :     | + ! :     1 _xzm_free  (in libsystem_malloc.dylib) + 960  [0x1854b04d8]
                            + ! :     | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 1760  [0x1030254b0]  rename.rs:364
                            + ! :     | + !   1 _RINvNtCs3bJEMBXHbwl_4core3ptr9drop_glueINtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map8BTreeMapjjEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 64  [0x102f00c44]  mod.rs:847
                            + ! :     | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            + ! :     | + ! 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + ! :     | + !   1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + ! :     | + !     1 DYLD-STUB$$memcmp  (in deslop) + 4  [0x10329cd9c]
                            + ! :     | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 296  [0x102f0c4e8]  content.rs:257
                            + ! :     | + ! 1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + ! :     | + !   1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 344  [0x102fdaedc]  map.rs:1942
                            + ! :     | + 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 336  [0x102f0c510]  content.rs:257
                            + ! :     | +   1 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 708  [0x102fa1778]  rename.rs:182
                            + ! :     | +     1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 856  [0x102fa9438]  corroboration.rs:26
                            + ! :     | +       1 _RINvMsi_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB6_8BTreeMapyyE27bulk_build_from_sorted_iterINtNtBc_3vec3VecTyyEEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 688  [0x102ef5c50]  map.rs:1716
                            + ! :     | +         1 _xzm_free  (in libsystem_malloc.dylib) + 1144  [0x1854b0590]
                            + ! :     | 7 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1812  [0x102fae4d4]  pair_compare.rs:153
                            + ! :     | + 4 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 228  [0x102ef2434]  cache.rs:139
                            + ! :     | + ! 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
                            + ! :     | + ! : 2 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            + ! :     | + ! :   1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            + ! :     | + ! :   | 1 _platform_memcmp  (in libsystem_platform.dylib) + 0  [0x18569af10]
                            + ! :     | + ! :   1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 924  [0x10303ff9c]  literal_echo.rs:164
                            + ! :     | + ! :     1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 116  [0x10327d7e4]
                            + ! :     | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9316  [0x102f0e824]  content.rs:264
                            + ! :     | + ! : 1 _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
                            + ! :     | + ! :   1 mach_absolute_time  (in libsystem_kernel.dylib) + 108  [0x18565010c]
                            + ! :     | + ! 1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9668  [0x102f0e984]  content.rs:264
                            + ! :     | + !   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            + ! :     | + !     1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 764  [0x102fa93dc]  corroboration.rs:26
                            + ! :     | + !       1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + ! :     | + !         1 _xzm_free  (in libsystem_malloc.dylib) + 20  [0x1854b012c]
                            + ! :     | + 2 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 84  [0x102ef23a4]  cache.rs:139
                            + ! :     | + ! 2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 76  [0x102f1040c]  content.rs:315
                            + ! :     | + !   2 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 172  [0x103023c48]  call_targets.rs:292
                            + ! :     | + !     1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 360  [0x102fa9248]  corroboration.rs:26
                            + ! :     | + !     : 1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            + ! :     | + !     :   1 _xzm_free  (in libsystem_malloc.dylib) + 1144  [0x1854b0590]
                            + ! :     | + !     1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 76  [0x102fa912c]  corroboration.rs:26
                            + ! :     | + 1 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 196  [0x102ef2414]  cache.rs:139
                            + ! :     | +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 496  [0x103024a30]  core_frontier.rs:151
                            + ! :     | +     1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent6append  (in deslop) + 248  [0x102fcfd38]  core_frontier.rs:98
                            + ! :     | +       1 _xzm_free  (in libsystem_malloc.dylib) + 220  [0x1854b01f4]
                            + ! :     | 4 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1372  [0x102fae31c]  pair_compare.rs:153
                            + ! :     | + 4 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 80  [0x102fbccac]  core.rs:422
                            + ! :     | +   4 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer23emitted_single_endpoint  (in deslop) + 208  [0x102fbcf70]  core.rs:436
                            + ! :     | +     3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + ! :     | +     ! 3 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + ! :     | +     !   1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 884  [0x102e272f8]  lib.rs:1206
                            + ! :     | +     !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 676  [0x102fdc304]  fingerprint.rs:282
                            + ! :     | +     !   : 1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 344  [0x102e26bb8]  lib.rs:581
                            + ! :     | +     !   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2096  [0x102e27ee0]  portable.rs:83
                            + ! :     | +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     | +                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            + ! :     | +                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            + ! :     | +                               1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2692  [0x102e28134]  portable.rs:83
                            + ! :     | 2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 580  [0x102fae004]  pair_compare.rs:108
                            + ! :     | + 2 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer7overlap  (in deslop) + 44  [0x102f96ea4]  overlap.rs:209
                            + ! :     | +   1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer9view_pair  (in deslop) + 32  [0x102f97150]  overlap.rs:410
                            + ! :     | +   ! 1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 1500  [0x102f96570]  overlap.rs:441
                            + ! :     | +   !   1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRNtNtCs8Hh7wWMBHrQ_5alloc6string6StringECs7J0RnZC7BVr_11deslop_core  (in deslop) + 76  [0x102f86640]  mod.rs:700
                            + ! :     | +   !     1 _RNvXs3_NtNtCs3bJEMBXHbwl_4core4hash3sipINtB5_6HasherNtB5_11Sip13RoundsENtB7_6Hasher5writeCs7J0RnZC7BVr_11deslop_core  (in deslop) + 12  [0x1030603b0]  sip.rs:262
                            + ! :     | +   1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer9view_pair  (in deslop) + 52  [0x102f97164]  overlap.rs:410
                            + ! :     | +     1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 1500  [0x102f96570]  overlap.rs:441
                            + ! :     | +       1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRNtNtCs8Hh7wWMBHrQ_5alloc6string6StringECs7J0RnZC7BVr_11deslop_core  (in deslop) + 76  [0x102f86640]  mod.rs:700
                            + ! :     | +         1 _RNvXs3_NtNtCs3bJEMBXHbwl_4core4hash3sipINtB5_6HasherNtB5_11Sip13RoundsENtB7_6Hasher5writeCs7J0RnZC7BVr_11deslop_core  (in deslop) + 376  [0x10306051c]  sip.rs:282
                            + ! :     | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1968  [0x102fae570]  pair_compare.rs:105
                            + ! :     | + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core13report_render23canonicalise_whitespace  (in deslop) + 184  [0x102fdd510]  report_render.rs:0
                            + ! :     | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 2088  [0x102fae5e8]  pair_compare.rs:105
                            + ! :     | + 1 _free  (in libsystem_malloc.dylib) + 36  [0x1854bfcf4]
                            + ! :     | 1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 664  [0x102fae058]  pair_compare.rs:110
                            + ! :     |   1 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 844  [0x102eecddc]  cache.rs:40
                            + ! :     |     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens16collapsed_leaves  (in deslop) + 48  [0x102fee540]  tokens.rs:198
                            + ! :     |       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            + ! :     |                                                                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 44  [0x102fee8a8]  tokens.rs:158
                            + ! :     1 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 152  [0x102fd4eec]  cluster_kind.rs:221
                            + ! :       1 _RNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare13text_identity30same_source_bytes_and_language  (in deslop) + 320  [0x103042db4]  text_identity.rs:17
                            + ! 2 _RNvXs1_NtNtNtCs3bJEMBXHbwl_4core4iter8adapters7flattenINtB5_7FlatMapINtNtNtBb_5slice4iter4IterINtNtCs8Hh7wWMBHrQ_5alloc3vec3VecjEEIB1v_B1u_ENCNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information22family_recovery_groupss_0ENtNtNtB9_6traits8iterator8Iterator4nextB2q_  (in deslop) + 464,468  [0x1030570d8,0x1030570dc]  flatten.rs:64
                            + 30 _RNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information22family_recovery_groups  (in deslop) + 288  [0x1030441f4]  information.rs:215
                            +   16 _RNvXs1_NtNtNtCs3bJEMBXHbwl_4core4iter8adapters7flattenINtB5_7FlatMapINtNtNtBb_5slice4iter4IterINtNtCs8Hh7wWMBHrQ_5alloc3vec3VecjEEIB1v_B1u_ENCNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information22family_recovery_groupss_0ENtNtNtB9_6traits8iterator8Iterator4nextB2q_  (in deslop) + 900  [0x10305728c]  flatten.rs:64
                            +   : 16 _RNvXs0_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB5_19ClusterKindMeasurerNtNtBd_7cluster16ClusterKindJudge4kind  (in deslop) + 288  [0x103055160]  cluster_kind.rs:352
                            +   :   16 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer9pair_kind  (in deslop) + 456  [0x102fd595c]  cluster_kind.rs:203
                            +   :     16 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 1204  [0x102fd5308]  cluster_kind.rs:225
                            +   :       7 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1372  [0x102fae31c]  pair_compare.rs:153
                            +   :       | 5 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 80  [0x102fbccac]  core.rs:422
                            +   :       | + 5 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer23emitted_single_endpoint  (in deslop) + 208  [0x102fbcf70]  core.rs:436
                            +   :       | +   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            +   :       | +   ! 3 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            +   :       | +   !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            +   :       | +   !   : 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            +   :       | +   !   :   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 616  [0x102e27918]  portable.rs:83
                            +   :       | +   !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 756  [0x102fdc354]  fingerprint.rs:282
                            +   :       | +   !   : 1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher12final_output  (in deslop) + 380  [0x102e26d3c]
                            +   :       | +   !   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1792  [0x102e27db0]  portable.rs:83
                            +   :       | +   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +     2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +         2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +           2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +             2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +               2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                 2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                     2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                         2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                           2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            +   :       | +                             : 1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            +   :       | +                             :   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 3476  [0x102e28444]  portable.rs:83
                            +   :       | +                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +                                                                                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            +   :       | +                                                                                         1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            +   :       | +                                                                                           1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 884  [0x102e272f8]  lib.rs:1206
                            +   :       | 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 100  [0x102fbccc0]  core.rs:423
                            +   :       | + 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer23emitted_single_endpoint  (in deslop) + 208  [0x102fbcf70]  core.rs:436
                            +   :       | +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +   :       | +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
                            +   :       | +         1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
                            +   :       | +           1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            +   :       | +             1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 672  [0x102e27950]  portable.rs:83
                            +   :       | 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 176  [0x102fbcd0c]  core.rs:427
                            +   :       |   1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1196  [0x102e27b5c]  portable.rs:83
                            +   :       5 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1812  [0x102fae4d4]  pair_compare.rs:153
                            +   :       | 3 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 228  [0x102ef2434]  cache.rs:139
                            +   :       | + 3 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
                            +   :       | +   2 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            +   :       | +   ! 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            +   :       | +   ! : 1 _platform_memcmp  (in libsystem_platform.dylib) + 236  [0x18569affc]
                            +   :       | +   ! 1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 756  [0x10303fef4]  literal_echo.rs:164
                            +   :       | +   !   1 _platform_memmove  (in libsystem_platform.dylib) + 428  [0x18569d50c]
                            +   :       | +   1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1100  [0x102f1f548]  literal_echo.rs:45
                            +   :       | 2 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 196  [0x102ef2414]  cache.rs:139
                            +   :       |   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 196  [0x103024904]  core_frontier.rs:149
                            +   :       |   ! 1 _RNvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontierNtNtB6_8frontier13MemberContent5slice  (in deslop) + 892  [0x102fcf868]  core_frontier.rs:63
                            +   :       |   !   1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            +   :       |   !     1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            +   :       |   !       1 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            +   :       |   !         1 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            +   :       |   !           1 xzm_realloc  (in libsystem_malloc.dylib) + 904  [0x1854aff0c]
                            +   :       |   !             1 _xzm_free  (in libsystem_malloc.dylib) + 924  [0x1854b04b4]
                            +   :       |   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content13core_frontier14joined_content  (in deslop) + 496  [0x103024a30]  core_frontier.rs:151
                            +   :       |     1 _xzm_free  (in libsystem_malloc.dylib) + 352  [0x1854b0278]
                            +   :       |       1 _platform_memset  (in libsystem_platform.dylib) + 180  [0x18569d144]
                            +   :       4 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 740  [0x102fae0a4]  pair_compare.rs:116
                            +   :         2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 336  [0x102f0c510]  content.rs:257
                            +   :         + 2 _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 708  [0x102fa1778]  rename.rs:182
                            +   :         +   1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 416  [0x102fa9280]  corroboration.rs:26
                            +   :         +   1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1188  [0x102fa9584]  corroboration.rs:27
                            +   :         +     1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            +   :         +       1 _xzm_free  (in libsystem_malloc.dylib) + 684  [0x1854b03c4]
                            +   :         1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            +   :         + 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 100  [0x102f1f160]  literal_echo.rs:41
                            +   :         +   1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17substituted_pairs  (in deslop) + 188  [0x1030257d8]  rename.rs:374
                            +   :         +     1 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            +   :         +       1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            +   :         +         1 _realloc  (in libsystem_malloc.dylib) + 252  [0x1854bfe4c]
                            +   :         +           1 xzm_malloc_zone_size  (in libsystem_malloc.dylib) + 32  [0x1854ba52c]
                            +   :         1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
                            +   :           1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            +   :             1 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
                            +   :               1 _platform_memcmp  (in libsystem_platform.dylib) + 236  [0x18569affc]
                            +   14 _RNvXs1_NtNtNtCs3bJEMBXHbwl_4core4iter8adapters7flattenINtB5_7FlatMapINtNtNtBb_5slice4iter4IterINtNtCs8Hh7wWMBHrQ_5alloc3vec3VecjEEIB1v_B1u_ENCNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information22family_recovery_groupss_0ENtNtNtB9_6traits8iterator8Iterator4nextB2q_  (in deslop) + 528  [0x103057118]  flatten.rs:64
                            +     14 _RNvXs0_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB5_19ClusterKindMeasurerNtNtBd_7cluster16ClusterKindJudge4kind  (in deslop) + 288  [0x103055160]  cluster_kind.rs:352
                            +       14 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer9pair_kind  (in deslop) + 456  [0x102fd595c]  cluster_kind.rs:203
                            +         14 _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 1204  [0x102fd5308]  cluster_kind.rs:225
                            +           5 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1372  [0x102fae31c]  pair_compare.rs:153
                            +           | 4 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 80  [0x102fbccac]  core.rs:422
                            +           | + 4 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer23emitted_single_endpoint  (in deslop) + 208  [0x102fbcf70]  core.rs:436
                            +           | +   3 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            +           | +   ! 3 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            +           | +   !   2 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1280,2548  [0x102e27bb0,0x102e280a4]  portable.rs:83
                            +           | +   !   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 960  [0x102fdc420]  fingerprint.rs:282
                            +           | +   !     1 _RNvMs4_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB5_6RawVecAhj20_E8grow_oneCs7J0RnZC7BVr_11deslop_core  (in deslop) + 60  [0x10327ec6c]  mod.rs:188
                            +           | +   !       1 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 280  [0x10327f9a4]
                            +           | +   !         1 DYLD-STUB$$malloc  (in deslop) + 4  [0x10329cd90]
                            +           | +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           | +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           | +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           | +         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           | +           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           | +             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           | +               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            +           | +                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            +           | +                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 676  [0x102fdc304]  fingerprint.rs:282
                            +           | +                     1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 852  [0x102e272d8]  lib.rs:1205
                            +           | +                       1 _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 168  [0x102e26b08]  lib.rs:563
                            +           | 1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 196  [0x102fbcd20]  core.rs:427
                            +           |   1 _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer8resolved  (in deslop) + 532  [0x102fbd248]  core.rs:453
                            +           |     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            +           |       1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            +           |         1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 1252  [0x102e27b94]  portable.rs:83
                            +           3 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1812  [0x102fae4d4]  pair_compare.rs:153
                            +           | 3 _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 228  [0x102ef2434]  cache.rs:139
                            +           |   2 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            +           |   + 2 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
                            +           |   +   2 _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 924  [0x10303ff9c]  literal_echo.rs:164
                            +           |   +     2 _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
                            +           |   +       2 _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
                            +           |   +         2 _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
                            +           |   +           2 _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
                            +           |   +             1 _xzm_free  (in libsystem_malloc.dylib) + 1568  [0x1854b0738]
                            +           |   +             1 xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
                            +           |   +               1 _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 256  [0x1854b503c]
                            +           |   1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9668  [0x102f0e984]  content.rs:264
                            +           |     1 _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
                            +           |       1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1188  [0x102fa9584]  corroboration.rs:27
                            +           |         1 _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
                            +           |           1 DYLD-STUB$$free  (in deslop) + 4  [0x10329cc94]
                            +           2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 580  [0x102fae004]  pair_compare.rs:108
                            +           | 2 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer7overlap  (in deslop) + 44  [0x102f96ea4]  overlap.rs:209
                            +           |   1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer9view_pair  (in deslop) + 32  [0x102f97150]  overlap.rs:410
                            +           |   + 1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 2128  [0x102f967e4]  overlap.rs:441
                            +           |   +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint20collect_fingerprints  (in deslop) + 588  [0x102fdc7d4]  fingerprint.rs:37
                            +           |   +     1 _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
                            +           |   +       1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2456  [0x102e28048]  portable.rs:83
                            +           |   1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer9view_pair  (in deslop) + 52  [0x102f97164]  overlap.rs:410
                            +           |     1 _RNvMNtCs7J0RnZC7BVr_11deslop_core7overlapNtB2_15OverlapMeasurer4view  (in deslop) + 516  [0x102f96198]  overlap.rs:441
                            +           |       1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           |         1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           |           1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           |             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           |               1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           |                 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           |                   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
                            +           |                     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
                            +           |                       1 _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
                            +           |                         1 _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 2576  [0x102e280c0]  portable.rs:83
                            +           2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 664  [0x102fae058]  pair_compare.rs:110
                            +           | 2 _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 844  [0x102eecddc]  cache.rs:40
                            +           |   2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens16collapsed_leaves  (in deslop) + 116  [0x102fee584]  tokens.rs:202
                            +           |     2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            +           |       2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            +           |         2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            +           |           2 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            +           |             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            +           |             + 1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            +           |             +   1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
                            +           |             +     1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 68  [0x102fee650]  tokens.rs:235
                            +           |             +       1 _RNvNtCs7J0RnZC7BVr_11deslop_core11boilerplate29is_import_boilerplate_carrier  (in deslop) + 580  [0x102fdb754]  boilerplate.rs:31
                            +           |             1 _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 316  [0x102fee748]  tokens.rs:250
                            +           2 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 740  [0x102fae0a4]  pair_compare.rs:116
                            +             1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
                            +             + 1 _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 116  [0x102f1f170]  literal_echo.rs:41
                            +             +   1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1368  [0x102fa9638]  corroboration.rs:27
                            +             +     1 _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryyTyjEE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 32  [0x102fc1720]  entry.rs:145
                            +             1 _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9468  [0x102f0e8bc]  content.rs:264
                            1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6renderNtB4_15PipelineSession19partition_and_split  (in deslop) + 688  [0x102fb0634]  render.rs:184
                            + 1 _RNvNtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6render11information16has_missing_pair  (in deslop) + 328  [0x10304313c]  information.rs:246
                            +   1 _RNvMs1_NtCsdnfh2UXVlgZ_9hashbrown3mapINtB5_7HashMapTjjEuNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateE6insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 48  [0x102fc0cf8]  map.rs:1807
                            +     1 _RINvYNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateNtNtCs3bJEMBXHbwl_4core4hash11BuildHasher8hash_oneRTjjEECs7J0RnZC7BVr_11deslop_core  (in deslop) + 108  [0x102f86ff0]  mod.rs:700
                            +       1 _RNvXs3_NtNtCs3bJEMBXHbwl_4core4hash3sipINtB5_6HasherNtB5_11Sip13RoundsENtB7_6Hasher5writeCs7J0RnZC7BVr_11deslop_core  (in deslop) + 364  [0x103060510]  sip.rs:289
                            1 _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session6renderNtB4_15PipelineSession19partition_and_split  (in deslop) + 1004  [0x102fb0770]  render.rs:184
    
    Total number in stack (recursive counted multiple, when >=5):
            363       _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 628  [0x102feeaf0]  tokens.rs:164
            41       _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop) + 0  [0x102e276b0]  portable.rs:83
            30       _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 500  [0x102feea70]  tokens.rs:164
            29       _xzm_free  (in libsystem_malloc.dylib) + 0  [0x1854b0118]
            28       _RNvNtCs7J0RnZC7BVr_11deslop_core7sibling21window_hash_for_nodes  (in deslop) + 164  [0x102ff03f0]  sibling.rs:270
            24       _platform_memmove  (in libsystem_platform.dylib) + 0  [0x18569d360]
            20       _RNvMs5_NtCs8Hh7wWMBHrQ_5alloc7raw_vecNtB5_11RawVecInner11finish_growCs7J0RnZC7BVr_11deslop_core  (in deslop) + 252  [0x10327f988]
            20       _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens25resolve_fingerprint_nodes  (in deslop) + 136  [0x102fee904]  tokens.rs:161
            19       _RINvNvMs2_NtCs8Hh7wWMBHrQ_5alloc7raw_vecINtB8_11RawVecInnerpE7reserve21do_reserve_and_handleNtNtBa_5alloc6GlobalECs7J0RnZC7BVr_11deslop_core  (in deslop) + 100  [0x10327d7d4]
            18       _realloc  (in libsystem_malloc.dylib) + 452  [0x1854bff14]
            17       _malloc_zone_realloc  (in libsystem_malloc.dylib) + 148  [0x1854bf6f8]
            17       _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib) + 0  [0x1854b4f3c]
            16       _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 616  [0x102e271ec]  lib.rs:1205
            14       _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 328  [0x102fdaecc]  map.rs:1936
            13       _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 1164  [0x102f1f588]  literal_echo.rs:51
            12       _RNvNtCs7J0RnZC7BVr_11deslop_core6tokens24collect_collapsed_leaves  (in deslop) + 136  [0x102fee694]  tokens.rs:240
            11       _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop) + 76  [0x103024e1c]  rename.rs:330
            11       _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 172  [0x10303fcac]  literal_echo.rs:154
            10       _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 724  [0x102fdc334]  fingerprint.rs:282
            10       _platform_memcmp  (in libsystem_platform.dylib) + 0  [0x18569af10]
            10       _xzm_free  (in libsystem_malloc.dylib) + 1048  [0x1854b0530]
            10       mach_absolute_time  (in libsystem_kernel.dylib) + 0  [0x1856500a0]
            9       _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 248  [0x102f0c4b8]  content.rs:257
            9       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 0  [0x102fa90e0]  corroboration.rs:26
            9       _RNvMs_NtCs7J0RnZC7BVr_11deslop_core3astNtB4_14NormalizedNode18subtree_node_count  (in deslop) + 40  [0x102fc7488]  ast.rs:136
            8       _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 504  [0x102f0c5b8]  content.rs:264
            8       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 0  [0x102fa90e0]  corroboration.rs:27
            8       xzm_realloc  (in libsystem_malloc.dylib) + 808  [0x1854afeac]
            7       _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 9668  [0x102f0e984]  content.rs:264
            7       _RINvNtCs7J0RnZC7BVr_11deslop_core7content18pair_contradictionNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 76  [0x102f1040c]  content.rs:315
            7       _RINvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo14literal_echoesNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB8_  (in deslop) + 116  [0x102f1f170]  literal_echo.rs:41
            7       _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer23emitted_single_endpoint  (in deslop) + 208  [0x102fbcf70]  core.rs:436
            7       _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 924  [0x10303ff9c]  literal_echo.rs:164
            6       <deduplicated_symbol>  (in libsystem_malloc.dylib) + 0  [0x1854af15c]
            6       _RINvNtCs7J0RnZC7BVr_11deslop_core7content13pair_evidenceNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB4_  (in deslop) + 336  [0x102f0c510]  content.rs:257
            6       _RNvMNtNtCs7J0RnZC7BVr_11deslop_core7content6renameNtB2_15LiteralEvidence7measure  (in deslop) + 708  [0x102fa1778]  rename.rs:182
            6       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 360  [0x102fa9248]  corroboration.rs:26
            6       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 856  [0x102fa9438]  corroboration.rs:26
            6       _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 76  [0x102e26aac]  lib.rs:546
            6       _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop) + 0  [0x102fdad84]  map.rs:1942
            6       _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 0  [0x102fdc060]  fingerprint.rs:282
            6       _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop) + 676  [0x102fdc304]  fingerprint.rs:282
            6       _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename17substituted_pairs  (in deslop) + 188  [0x1030257d8]  rename.rs:374
            6       _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop) + 0  [0x10303fc00]  literal_echo.rs:154
            6       xzm_realloc  (in libsystem_malloc.dylib) + 0  [0x1854afb84]
            6       xzm_realloc  (in libsystem_malloc.dylib) + 904  [0x1854aff0c]
            5       DYLD-STUB$$memcmp  (in deslop) + 0  [0x10329cd98]
            5       _RINvMNtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB3_15ContentMeasurer6memberNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateB1f_EB7_  (in deslop) + 844  [0x102eecddc]  cache.rs:40
            5       _RINvMs_NtNtCs7J0RnZC7BVr_11deslop_core7content5cacheNtB5_11ContentPair4coreNtNtNtCsdnanKjkF2cC_3std4hash6random11RandomStateEB9_  (in deslop) + 228  [0x102ef2434]  cache.rs:139
            5       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 1188  [0x102fa9584]  corroboration.rs:27
            5       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop) + 244  [0x102fa91d4]  corroboration.rs:26
            5       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1372  [0x102fae31c]  pair_compare.rs:153
            5       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 1812  [0x102fae4d4]  pair_compare.rs:153
            5       _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compareNtB4_15PipelineSession12measure_axes  (in deslop) + 740  [0x102fae0a4]  pair_compare.rs:116
            5       _RNvMs0_NtNtCs7J0RnZC7BVr_11deslop_core7overlap4coreNtB7_15OverlapMeasurer12aligned_core  (in deslop) + 80  [0x102fbccac]  core.rs:422
            5       _RNvMs2_NtNtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3map5entryINtB5_5EntryTyyEjE9or_insertCs7J0RnZC7BVr_11deslop_core  (in deslop) + 464  [0x102fc11d0]  entry.rs:147
            5       _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer13classify_pair  (in deslop) + 1204  [0x102fd5308]  cluster_kind.rs:225
            5       _RNvMs_NtNtNtNtCs7J0RnZC7BVr_11deslop_core8pipeline7session12pair_compare12cluster_kindNtB4_19ClusterKindMeasurer9pair_kind  (in deslop) + 456  [0x102fd595c]  cluster_kind.rs:203
            5       _RNvMsa_Cs98dNin8E3s3_6blake3NtB5_10ChunkState6update  (in deslop) + 320  [0x102e26ba0]  lib.rs:577
            5       _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop) + 0  [0x102e26f84]  lib.rs:1206
            5       _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content12call_targets17changed_positions  (in deslop) + 172  [0x103023c48]  call_targets.rs:292
    
    Sort by top of stack, same collapsed (when >= 5):
            _RNvNtCs98dNin8E3s3_6blake38portable17compress_in_place  (in deslop)        49
            _xzm_free  (in libsystem_malloc.dylib)        30
            _platform_memcmp  (in libsystem_platform.dylib)        27
            _platform_memmove  (in libsystem_platform.dylib)        26
            _RNvMNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename13corroborationNtB2_14ModalBijection4over  (in deslop)        22
            _xzm_xzone_malloc_tiny  (in libsystem_malloc.dylib)        20
            DYLD-STUB$$memcmp  (in deslop)        12
            _RNvNtNtNtCs7J0RnZC7BVr_11deslop_core7content6rename12literal_echo16replaced_matches  (in deslop)        12
            _RNvMsz_NtNtNtCs8Hh7wWMBHrQ_5alloc11collections5btree3mapINtB5_8IntoIterTyyEjE10dying_nextCs7J0RnZC7BVr_11deslop_core  (in deslop)        11
            _RNvMsd_Cs98dNin8E3s3_6blake3NtB5_6Hasher6update  (in deslop)        10
            mach_absolute_time  (in libsystem_kernel.dylib)        10
            _RNvNtNtCs7J0RnZC7BVr_11deslop_core7content6rename14rename_mapping  (in deslop)        7
            <deduplicated_symbol>  (in libsystem_malloc.dylib)        6
            _RNvNtCs7J0RnZC7BVr_11deslop_core11fingerprint12subtree_hash  (in deslop)        6
            _RNvXs3_NtNtCs3bJEMBXHbwl_4core4hash3sipINtB5_6HasherNtB5_11Sip13RoundsENtB7_6Hasher5writeCs7J0RnZC7BVr_11deslop_core  (in deslop)        6
            xzm_realloc  (in libsystem_malloc.dylib)        6
            _RNvNtCs7J0RnZC7BVr_11deslop_core13report_render23canonicalise_whitespace  (in deslop)        5
            _free  (in libsystem_malloc.dylib)        5
    
