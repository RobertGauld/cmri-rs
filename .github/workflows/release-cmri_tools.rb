# frozen_string_literal: true

require 'zip'

ZIP_FILE = ARGV.fetch(0)
LABEL = ARGV.fetch(1)

zip = Zip::File.new(ZIP_FILE, create: true, compression_level: Zlib::BEST_COMPRESSION)

# ../target/#{matrix.target}/#{debug|release}/.........
TARGET = File.join('..', 'target', '*', '*')

# Add executables
puts 'Adding executables'
Dir.glob(File.join(TARGET, '*')).each do |file|
  next unless File.file?(file) && File.executable?(file) && ['', '.exe'].include?(File.extname(file))
  puts "\t#{File.basename file}"
  zip.add File.basename(file), file
end

# Add auto complete files
puts 'Adding autocompletes'
Dir.glob(File.join(TARGET, 'build', '*', 'out', 'autocomplete', '*')).each do |dir|
  puts "\t#{File.basename dir}"
  Dir.glob(File.join(dir, '*')).each do |file|
    puts "\t\t#{File.basename file}"
    zip.add File.join('autocomplete', File.basename(dir), File.basename(file)), file
  end
end

puts "Saving zip #{ZIP_FILE}"
zip.close

if LABEL
  puts "Uploading zip \"#{ZIP_FILE}\" with label \"#{LABEL}\""
  system "gh release upload  --repo \"#{ENV.fetch('GITHUB_REPOSITORY')}\" \"#{ENV.fetch('GITHUB_REF_NAME')}\" \"#{ZIP_FILE}\"#\"#{LABEL}\""
end
