<?php
$directory = __DIR__ . '/stdlib';
$outputFile = __DIR__ . '/content.txt';

if (!is_dir($directory)) {
  die("Directory does not exist: $directory\n");
}

// Get all PHP files in the directory
$files = glob($directory . '/*.xen');

if (empty($files)) {
  file_put_contents($outputFile, "No ?? files found in $directory\n");
  exit;
}

// Initialize output string
$output = "";

foreach ($files as $file) {
  $filename = basename($file); // just the file name with extension
  $content = file_get_contents($file);
  $output .= $filename . "\n" . $content . "\n\n"; // keep the format
}

// Write everything to content.txt
file_put_contents($outputFile, $output);

echo "All file contents have been saved to $outputFile\n";
