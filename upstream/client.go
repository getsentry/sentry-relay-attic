package upstream

import (
	"bytes"
	"io/ioutil"
	"log"
	"net/http"
	"net/url"
	"sync"
)

// An upstream client
type Client struct {
	*http.Client
	base *url.URL
}

// Join a base url.URL and with a path.
// returns a new url.URL object
func join(base *url.URL, path string) *url.URL {
	return &url.URL{
		Host:   base.Host,
		Scheme: base.Scheme,
		Path:   path,
	}
}

// Send a message off to the upstream server
func (c *Client) Send(path, auth string, body []byte) {
	req := requestPool.Get().(*http.Request)
	defer requestPool.Put(req)

	req.Method = "POST"
	req.URL = join(c.base, path)
	req.Proto = "HTTP/1.1"
	req.Header = make(http.Header)
	req.Host = c.base.Host
	req.Body = ioutil.NopCloser(bytes.NewReader(body))
	req.ContentLength = int64(len(body))
	req.Header.Add("X-Sentry-Auth", auth)

	resp, err := c.Client.Do(req)
	if err != nil {
		log.Println(err)
		return
	}
	if resp.StatusCode != http.StatusOK {
		log.Println(resp)
	}
}

// Create a new instance of a Client
func New(addr string) (*Client, error) {
	u, err := url.Parse(addr)
	if err != nil {
		return nil, err
	}
	return &Client{
		Client: &http.Client{},
		base: &url.URL{
			Scheme: u.Scheme,
			Host:   u.Host,
		},
	}, nil
}

// A pool to reuse http.Request objects efficiently
var requestPool = sync.Pool{
	New: func() interface{} {
		return &http.Request{}
	},
}
