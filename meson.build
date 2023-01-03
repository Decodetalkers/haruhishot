project('haruhishot', 'rust',
  version: '0.2',
  meson_version: '>= 0.60'
)


dependency('wayland-client')
dependency('wlroots')

cargo = find_program('cargo', version: '>= 1.65')

rustc = find_program('rustc', version: '>= 1.65')

if get_option('debug')
  command = [
    cargo, 'build',
    '&&',
    'cp', meson.global_source_root() / 'target' / 'debug' / meson.project_name(), '@OUTPUT@'
  ]
else
  command = [
    cargo, 'build',
    '--release',
    '&&',
    'cp', meson.global_source_root() / 'target' / 'release' / meson.project_name(), '@OUTPUT@'
  ]
endif

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')

gdgbattersaver_target = custom_target(meson.project_name(),
  output: meson.project_name(),
  build_by_default: true,
  install: true,
  install_dir: bindir,
  console: true,
  command: command
)
