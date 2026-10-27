project('shredder', 'c', version: '3.1.0')
gnome = import('gnome')
deps = [
                dependency('gtk4', 
                          version: '>=4.18.0',
                          ),
                dependency('libadwaita-1', version: '>=1.7.0'),
                dependency("gio-unix-2.0"),
]
blueprint = find_program('blueprint-compiler')
subdir('src')
subdir('data')
