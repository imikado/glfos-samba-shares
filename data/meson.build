subdir('icons')

gnome = import('gnome')

configure_file(
  input: 'org.dupot.samba-shares.desktop.in',
  output: 'org.dupot.samba-shares.desktop.policy',
  configuration: conf,
  install_dir: datadir / 'polkit-1' / 'actions'
)

configure_file(
  input: 'org.dupot.samba-shares.desktop.in',
  output: 'org.dupot.samba-shares.desktop',
  configuration: conf,
  install_dir: datadir / 'applications'
)