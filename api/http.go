package api

import (
	"io/ioutil"
	"log"
	"net/http"
	"time"

	"github.com/mattrobenolt/emptygif"
	"github.com/mattrobenolt/size"

	"github.com/getsentry/sentry-relay/upstream"
)

type sentry struct {
	*upstream.Client
}

// Router for requests based on the method
func (s *sentry) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	switch r.Method {
	case "GET":
		s.HandleGET(w, r)
	case "POST":
		s.HandlePOST(w, r)
	default:
		http.Error(w, "405 method not allowed", http.StatusMethodNotAllowed)
	}
}

// Respond to GET requests with an empty pixel
func (s *sentry) HandleGET(w http.ResponseWriter, r *http.Request) {
	// TODO(mattrobenolt): Convert the querystring arguments into
	// a request to pass to s.Client.Send
	emptygif.Handle(w, r)
}

func (s *sentry) HandlePOST(w http.ResponseWriter, r *http.Request) {
	auth := r.Header.Get("X-Sentry-Auth")
	body, _ := ioutil.ReadAll(r.Body)

	w.Header().Set("Content-Type", "application/json")
	// TODO(mattrobenolt): Extract event id or generate one
	// We don't know the event id yet to return back anything useful
	w.Write([]byte(`{"id":""}`))

	go s.Client.Send(r.URL.Path, auth, body)
}

// Create a new http.Server bound with the sentry http handler
func New(addr, listen string) (*http.Server, error) {
	client, err := upstream.New(addr)
	if err != nil {
		return nil, err
	}
	return &http.Server{
		Addr:           listen,
		Handler:        &sentry{client},
		ReadTimeout:    100 * time.Millisecond,
		WriteTimeout:   100 * time.Millisecond,
		MaxHeaderBytes: int(4 * size.Kilobyte),
	}, nil
}

// Shortcut for creating an http server and listening
func ListenAndServe(upstream, listen string) error {
	s, err := New(upstream, listen)
	if err != nil {
		panic(err)
	}
	log.Println("ready.")
	return s.ListenAndServe()
}
