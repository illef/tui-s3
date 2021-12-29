function __s3_is_maybe_bucket
    commandline -ct | string match -qr -- '^(|s|s3|s3:?/?/?[^/]*)$'
end

function __s3_is_bucket
    commandline -ct | string match -q -r -- '^s3:/?/?[^/]*$'
end

function __s3_is_remote_path
    commandline -ct | string match -q -r -- "^s3://.+/.*"
end

function __s3_ls_buckets
    aws s3 ls | string replace -rf '.* (\S+)$' 's3://$1/'
end

function __s3_ls_dir
    set -l dir (commandline -ct | string replace -rf '(s3://.*/).*' '$1')
    printf "$dir%s\n" (aws s3 ls $dir 2>/dev/null | string replace -fr '^(:?\S+ +\S+ +\S+ |^.*PRE )(.*)' '$2')
end


complete -c tui-s3 -n "__s3_is_maybe_bucket" -xa "(__s3_ls_buckets)"
complete -c tui-s3 -n __s3_is_remote_path -xa "(__s3_ls_dir)"
complete -c tui-s3 -n __s3_is_bucket -xa "(__s3_ls_buckets)"
