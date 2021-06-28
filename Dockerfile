FROM scratch

# These commands copy your files into the specified directory in the image
# and set that as the working location
COPY target/x86_64-unknown-linux-musl/release/PatchyVideo-related-video-finder /webapp/app
WORKDIR /webapp

# This command runs your application, comment out this line to compile only
CMD ["./app"]
